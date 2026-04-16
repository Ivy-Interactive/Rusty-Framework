use crate::views::view::BuildContext;
use std::any::TypeId;
use std::sync::Arc;

/// Register a context value of type `T` for descendant views to access.
///
/// Ported from Ivy-Framework's `ViewContext` context system. The value is stored
/// in the HookStore's context map keyed by `TypeId`, making it available to
/// any descendant that calls `use_context::<T>()`.
pub fn create_context<T: Send + Sync + Clone + 'static>(ctx: &mut BuildContext, value: T) {
    let type_id = TypeId::of::<T>();
    ctx.store.contexts.insert(type_id, Arc::new(value));
}

/// Retrieve a context value of type `T` from the current view or an ancestor's context.
///
/// Walks the ancestor chain from the current view upward, checking each view's
/// HookStore for a matching context value. Panics if no context of type `T` is found.
pub fn use_context<T: Send + Sync + Clone + 'static>(ctx: &mut BuildContext) -> T {
    let _idx = ctx.next_hook_index();
    let type_id = TypeId::of::<T>();

    // Walk from current store up through ancestor stores
    ctx.find_ancestor_context(type_id)
        .and_then(|any| any.downcast_ref::<T>())
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

    #[test]
    fn test_ancestor_context_snapshot_isolation() {
        use crate::views::view::{Element, View};
        use crate::widgets::text::TextBlock;
        use std::sync::Mutex;

        static CHILD_RESULT: Mutex<Option<String>> = Mutex::new(None);

        struct ChildView;
        impl View for ChildView {
            fn build(&self, ctx: &mut BuildContext) -> Element {
                let theme: Theme = use_context(ctx);
                *CHILD_RESULT.lock().unwrap() = Some(theme.primary_color.clone());
                Element::Widget(Box::new(TextBlock::new(&theme.primary_color)))
            }
        }

        let mut parent_store = HookStore::new();
        let parent_view_id = uuid::Uuid::new_v4();
        let mut ctx = BuildContext::with_view_id(&mut parent_store, None, parent_view_id);

        // Parent creates context with "blue"
        create_context(
            &mut ctx,
            Theme {
                primary_color: "blue".to_string(),
            },
        );

        // Child reads the context — gets a snapshot
        let (_element, _child_id, _child_store) = ctx.child_view(ChildView, None);
        let child_saw = CHILD_RESULT.lock().unwrap().take().unwrap();
        assert_eq!(child_saw, "blue");

        // Parent modifies its context to "red" after child was built
        create_context(
            &mut ctx,
            Theme {
                primary_color: "red".to_string(),
            },
        );

        // Verify parent's store changed
        let parent_theme = ctx
            .store
            .contexts
            .get(&TypeId::of::<Theme>())
            .unwrap()
            .downcast_ref::<Theme>()
            .unwrap();
        assert_eq!(parent_theme.primary_color, "red");

        // Build another child — it should see "red" since snapshot is taken at child_view() time
        let (_element2, _child_id2, _child_store2) = ctx.child_view(ChildView, None);
        let child2_saw = CHILD_RESULT.lock().unwrap().take().unwrap();
        assert_eq!(child2_saw, "red");
    }
}
