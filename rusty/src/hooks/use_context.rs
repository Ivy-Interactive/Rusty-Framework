use crate::views::view::BuildContext;
use std::any::TypeId;

/// Register a context value of type `T` for descendant views to access.
///
/// Ported from Ivy-Framework's `ViewContext` context system. The value is stored
/// in the HookStore's context map keyed by `TypeId`, making it available to
/// any descendant that calls `use_context::<T>()`.
pub fn create_context<T: Send + Sync + Clone + 'static>(ctx: &mut BuildContext, value: T) {
    let type_id = TypeId::of::<T>();
    ctx.store.contexts.insert(type_id, Box::new(value));
}

/// Retrieve a context value of type `T` from an ancestor's context.
///
/// Panics if no context of type `T` has been created by an ancestor.
/// In a full multi-view implementation, this would walk the view ancestor chain;
/// currently it looks up the current view's HookStore.
pub fn use_context<T: Send + Sync + Clone + 'static>(ctx: &mut BuildContext) -> T {
    let _idx = ctx.next_hook_index();
    let type_id = TypeId::of::<T>();
    ctx.store
        .contexts
        .get(&type_id)
        .and_then(|boxed| boxed.downcast_ref::<T>())
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "No context found for type {}. Did you forget to call create_context?",
                std::any::type_name::<T>()
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;

    #[derive(Clone, Debug, PartialEq)]
    struct Theme {
        primary_color: String,
    }

    #[test]
    fn test_create_and_use_context() {
        let mut store = HookStore::new();
        let mut ctx = BuildContext::new(&mut store, None);

        let theme = Theme {
            primary_color: "blue".to_string(),
        };
        create_context(&mut ctx, theme.clone());

        let retrieved: Theme = use_context(&mut ctx);
        assert_eq!(retrieved, theme);
    }

    #[test]
    fn test_context_persists_across_builds() {
        let mut store = HookStore::new();

        // First build — create context
        {
            let mut ctx = BuildContext::new(&mut store, None);
            create_context(&mut ctx, 42i32);
        }

        // Second build — retrieve context
        {
            let mut ctx = BuildContext::new(&mut store, None);
            let val: i32 = use_context(&mut ctx);
            assert_eq!(val, 42);
        }
    }

    #[test]
    #[should_panic(expected = "No context found")]
    fn test_use_context_panics_without_create() {
        let mut store = HookStore::new();
        let mut ctx = BuildContext::new(&mut store, None);
        let _: Theme = use_context(&mut ctx);
    }
}
