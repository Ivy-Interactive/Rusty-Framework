use crate::hooks::use_state::State;
use crate::views::view::BuildContext;

/// A ref handle that wraps `State<T>` but does NOT trigger rebuilds on mutation.
///
/// Analogous to Ivy-Framework's `UseRef` which is `UseState(buildOnChange: false)`.
/// Useful for storing mutable values that shouldn't cause re-renders (e.g., DOM refs,
/// timers, previous values).
pub type Ref<T> = State<T>;

/// Create a mutable ref that persists across re-renders without triggering rebuilds.
///
/// Like `use_state`, the value persists in the HookStore. Unlike `use_state`,
/// calling `set()` or `update()` does NOT send a rebuild notification.
pub fn use_ref<T: Send + Sync + Clone + 'static>(ctx: &mut BuildContext, initial: T) -> Ref<T> {
    let idx = ctx.next_hook_index();
    ctx.store
        .get_or_init_state(idx, || State::new_silent(initial))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;

    #[test]
    fn test_use_ref_persists_value() {
        let mut store = HookStore::new();

        // First build
        {
            let mut ctx = BuildContext::new(&mut store, None);
            let r = use_ref(&mut ctx, 0);
            r.set(42);
        }

        // Second build — value persists
        {
            let mut ctx = BuildContext::new(&mut store, None);
            let r = use_ref::<i32>(&mut ctx, 0);
            assert_eq!(r.get(), 42);
        }
    }

    #[test]
    fn test_use_ref_does_not_trigger_rebuild() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let mut store = HookStore::new();
        let mut ctx = BuildContext::new(&mut store, Some(tx));
        let r = use_ref(&mut ctx, 0);

        r.set(99);

        // Should NOT have sent a rebuild notification
        assert!(rx.try_recv().is_err());
    }
}
