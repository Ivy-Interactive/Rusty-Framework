use crate::views::view::BuildContext;
use std::sync::Arc;

/// Resolve a server-level service of type `T` registered on the `RustyServer`.
///
/// Ported from Ivy-Framework's `ViewContext.UseService<T>`. Where `use_context` walks the
/// ancestor view chain and requires some ancestor to have called `create_context`, this
/// reads the server's `ServiceRegistry`, so any view can resolve a service no matter where
/// it sits in the tree. Register services with `RustyServer::with_service`.
///
/// Panics if no service of type `T` was registered, naming the type in the message —
/// the same failure mode as `use_context`.
///
/// **This hook does not consume a hook index**, unlike every other hook in this crate.
/// Resolution is keyed by type rather than by call order, so an index would buy nothing,
/// and burning one would make a conditional `use_service` call shift the indices of every
/// later `use_state`/`use_effect` in that view and desync the hook store.
pub fn use_service<T: Send + Sync + 'static>(ctx: &BuildContext) -> Arc<T> {
    ctx.services().get::<T>().unwrap_or_else(|| {
        panic!(
            "No service registered for type {}. Did you forget to call RustyServer::with_service?",
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
    use crate::hooks::use_state::use_state;
    use crate::views::view::{Element, View};
    use crate::widgets::text::TextBlock;

    struct Greeter {
        greeting: String,
    }

    struct Counter {
        start: i32,
    }

    fn registry_with_greeter(greeting: &str) -> Arc<ServiceRegistry> {
        let mut registry = ServiceRegistry::new();
        registry.register(Greeter {
            greeting: greeting.to_string(),
        });
        Arc::new(registry)
    }

    #[test]
    fn test_view_resolves_injected_service_during_build() {
        struct GreetingView;
        impl View for GreetingView {
            fn build(&self, ctx: &mut BuildContext) -> Element {
                let greeter = use_service::<Greeter>(ctx);
                Element::Widget(Box::new(TextBlock::new(&greeter.greeting)))
            }
        }

        let mut store = HookStore::new();
        let mut ctx = BuildContext::with_services(
            &mut store,
            None,
            uuid::Uuid::new_v4(),
            registry_with_greeter("hello from service"),
        );

        let element = GreetingView.build(&mut ctx);
        let json = serde_json::to_value(&element).unwrap().to_string();
        assert!(
            json.contains("hello from service"),
            "Expected service value in: {}",
            json
        );
    }

    #[test]
    #[should_panic(expected = "No service registered for type")]
    fn test_use_service_panics_without_registration() {
        let mut store = HookStore::new();
        let ctx = BuildContext::new(&mut store, None);
        let _ = use_service::<Greeter>(&ctx);
    }

    #[test]
    fn test_panic_message_names_the_missing_type() {
        let mut store = HookStore::new();
        let ctx = BuildContext::new(&mut store, None);

        let err = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = use_service::<Greeter>(&ctx);
        }))
        .expect_err("use_service should panic");

        let msg = err
            .downcast_ref::<String>()
            .map(|s| s.as_str())
            .unwrap_or_default();
        assert!(
            msg.contains("Greeter"),
            "Panic message should name the missing type, got: {}",
            msg
        );
    }

    #[test]
    fn test_try_use_service_returns_none_when_unregistered() {
        let mut store = HookStore::new();
        let ctx = BuildContext::new(&mut store, None);
        assert!(try_use_service::<Greeter>(&ctx).is_none());
    }

    #[test]
    fn test_service_resolves_inside_child_view() {
        struct ChildView;
        impl View for ChildView {
            fn build(&self, ctx: &mut BuildContext) -> Element {
                let greeter = use_service::<Greeter>(ctx);
                Element::Widget(Box::new(TextBlock::new(&format!(
                    "child: {}",
                    greeter.greeting
                ))))
            }
        }

        let mut store = HookStore::new();
        let mut ctx = BuildContext::with_services(
            &mut store,
            None,
            uuid::Uuid::new_v4(),
            registry_with_greeter("threaded through"),
        );

        // The parent never creates a context — the child reaches the registry directly,
        // proving the Arc threads through BuildContext::child_view.
        let (element, _child_id, _child_store) = ctx.child_view(ChildView, None);
        let json = serde_json::to_value(&element).unwrap().to_string();
        assert!(
            json.contains("child: threaded through"),
            "Expected child to resolve the service, got: {}",
            json
        );
    }

    #[test]
    fn test_service_resolves_in_nested_child_view() {
        struct GrandchildView;
        impl View for GrandchildView {
            fn build(&self, ctx: &mut BuildContext) -> Element {
                let greeter = use_service::<Greeter>(ctx);
                Element::Widget(Box::new(TextBlock::new(&format!(
                    "grandchild: {}",
                    greeter.greeting
                ))))
            }
        }

        struct ChildView;
        impl View for ChildView {
            fn build(&self, ctx: &mut BuildContext) -> Element {
                let (element, _id, _store) = ctx.child_view(GrandchildView, None);
                element
            }
        }

        let mut store = HookStore::new();
        let mut ctx = BuildContext::with_services(
            &mut store,
            None,
            uuid::Uuid::new_v4(),
            registry_with_greeter("two levels down"),
        );

        let (element, _child_id, _child_store) = ctx.child_view(ChildView, None);
        let json = serde_json::to_value(&element).unwrap().to_string();
        assert!(
            json.contains("grandchild: two levels down"),
            "Expected grandchild to resolve the service, got: {}",
            json
        );
    }

    #[test]
    fn test_use_service_between_use_state_calls_does_not_desync_hooks() {
        let mut registry = ServiceRegistry::new();
        registry.register(Counter { start: 7 });
        let services = Arc::new(registry);

        struct MixedView;
        impl View for MixedView {
            fn build(&self, ctx: &mut BuildContext) -> Element {
                let first = use_state(ctx, 1i32);
                // A service lookup sandwiched between two state hooks must not
                // consume an index, or `second` would read `first`'s slot.
                let counter = use_service::<Counter>(ctx);
                let second = use_state(ctx, 2i32);
                Element::Widget(Box::new(TextBlock::new(&format!(
                    "{}-{}-{}",
                    first.get(),
                    second.get(),
                    counter.start
                ))))
            }
        }

        let mut store = HookStore::new();
        let view_id = uuid::Uuid::new_v4();

        // First build seeds both state slots.
        {
            let mut ctx = BuildContext::with_services(&mut store, None, view_id, services.clone());
            let element = MixedView.build(&mut ctx);
            let json = serde_json::to_value(&element).unwrap().to_string();
            assert!(json.contains("1-2-7"), "First build produced: {}", json);
        }

        // Two distinct slots were allocated, not one shared slot.
        assert_eq!(
            store.states.len(),
            2,
            "Expected two independent state slots"
        );

        // Second build must read the same slots back in the same order.
        {
            let mut ctx = BuildContext::with_services(&mut store, None, view_id, services.clone());
            let element = MixedView.build(&mut ctx);
            let json = serde_json::to_value(&element).unwrap().to_string();
            assert!(json.contains("1-2-7"), "Second build produced: {}", json);
        }
    }

    #[test]
    fn test_repeated_use_service_calls_return_same_instance() {
        let mut store = HookStore::new();
        let ctx = BuildContext::with_services(
            &mut store,
            None,
            uuid::Uuid::new_v4(),
            registry_with_greeter("shared"),
        );

        let a = use_service::<Greeter>(&ctx);
        let b = use_service::<Greeter>(&ctx);
        assert!(Arc::ptr_eq(&a, &b));
    }
}
