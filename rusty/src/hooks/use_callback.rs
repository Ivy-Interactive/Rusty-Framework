use crate::hooks::deps::DynEq;
use crate::hooks::use_memo::use_memo;
use crate::views::view::BuildContext;
use std::sync::Arc;

/// Create a stable callback reference with dependency tracking.
///
/// Implemented as sugar over `use_memo` — returns the same `Arc<dyn Fn>` across
/// re-renders when deps haven't changed (same pattern as Ivy's `UseCallback.cs`).
pub fn use_callback<F, A>(
    ctx: &mut BuildContext,
    deps: &[&dyn DynEq],
    callback: F,
) -> Arc<dyn Fn(A) + Send + Sync>
where
    F: Fn(A) + Send + Sync + 'static,
    A: 'static,
{
    use_memo(ctx, deps, move || -> Arc<dyn Fn(A) + Send + Sync> {
        Arc::new(callback)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use std::sync::Mutex;

    #[test]
    fn test_use_callback_works() {
        let mut store = HookStore::new();
        let mut ctx = BuildContext::new(&mut store, None);
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        let cb = use_callback(&mut ctx, &[], move |_: ()| {
            *counter_clone.lock().unwrap() += 1;
        });

        cb(());
        cb(());
        assert_eq!(*counter.lock().unwrap(), 2);
    }

    #[test]
    fn test_use_callback_returns_same_arc_when_deps_unchanged() {
        let mut store = HookStore::new();
        let dep = 1i32;

        let ptr1: usize;
        let ptr2: usize;

        // First build
        {
            let mut ctx = BuildContext::new(&mut store, None);
            let cb = use_callback(&mut ctx, &[&dep as &dyn DynEq], |_: ()| {});
            ptr1 = Arc::as_ptr(&cb) as *const () as usize;
        }

        // Second build with same deps
        {
            let mut ctx = BuildContext::new(&mut store, None);
            let cb = use_callback(&mut ctx, &[&dep as &dyn DynEq], |_: ()| {});
            ptr2 = Arc::as_ptr(&cb) as *const () as usize;
        }

        assert_eq!(ptr1, ptr2, "Should return the same Arc pointer");
    }
}
