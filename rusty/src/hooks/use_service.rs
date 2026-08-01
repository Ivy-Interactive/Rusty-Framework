use std::sync::Arc;

use crate::views::view::BuildContext;

/// Resolve a shared service registered on the runtime's `ServiceRegistry`.
///
/// Ported from Ivy-Framework's `IViewContext.UseService<T>()`. Panics when the
/// service was never registered — like `use_context`, an unresolvable service is
/// a wiring bug, not a runtime condition.
///
/// Unlike most hooks this consumes no hook slot: resolution is a pure lookup with
/// no per-view state, so calling it conditionally cannot desync hook ordering.
pub fn use_service<T: Send + Sync + 'static>(ctx: &BuildContext) -> Arc<T> {
    ctx.services().get::<T>().unwrap_or_else(|| {
        panic!(
            "No service registered for type {}. Did you forget to register it on the ServiceRegistry?",
            std::any::type_name::<T>()
        )
    })
}

/// Resolve a server-level service of type `T`, or `None` when it was not registered.
///
/// The non-panicking counterpart to [`use_service`], for optional dependencies.
pub fn try_use_service<T: Send + Sync + 'static>(ctx: &BuildContext) -> Option<Arc<T>> {
    ctx.services().get::<T>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::services::ServiceRegistry;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::{Element, View};
    use crate::widgets::text::TextBlock;

    struct Greeter {
        greeting: String,
    }

    struct Unregistered;

    #[test]
    fn test_use_service_resolves_registered_service() {
        let services = Arc::new(ServiceRegistry::new());
        services.register(Arc::new(Greeter {
            greeting: "hello".to_string(),
        }));

        let mut store = HookStore::new();
        let ctx = BuildContext::new(&mut store, None).using_services(services);

        let greeter = use_service::<Greeter>(&ctx);
        assert_eq!(greeter.greeting, "hello");
    }

    #[test]
    #[should_panic(expected = "No service registered for type")]
    fn test_use_service_panics_with_type_name_when_missing() {
        let mut store = HookStore::new();
        let ctx = BuildContext::new(&mut store, None);
        let _ = use_service::<Unregistered>(&ctx);
    }

    #[test]
    fn test_try_use_service_returns_none_when_unregistered() {
        let mut store = HookStore::new();
        let ctx = BuildContext::new(&mut store, None);
        assert!(try_use_service::<Greeter>(&ctx).is_none());
    }

    #[test]
    fn test_child_context_inherits_registry() {
        use std::sync::Mutex;

        static CHILD_SAW: Mutex<Option<String>> = Mutex::new(None);

        struct ChildView;
        impl View for ChildView {
            fn build(&self, ctx: &mut BuildContext) -> Element {
                let greeter = use_service::<Greeter>(ctx);
                *CHILD_SAW.lock().unwrap() = Some(greeter.greeting.clone());
                Element::Widget(Box::new(TextBlock::new(&greeter.greeting)))
            }
        }

        let services = Arc::new(ServiceRegistry::new());
        services.register(Arc::new(Greeter {
            greeting: "from parent".to_string(),
        }));

        let mut store = HookStore::new();
        let mut ctx = BuildContext::with_view_id(&mut store, None, uuid::Uuid::new_v4())
            .using_services(services);

        let (_element, _child_id, _child_store) = ctx.child_view(ChildView, None);
        assert_eq!(CHILD_SAW.lock().unwrap().take().unwrap(), "from parent");
    }
}
