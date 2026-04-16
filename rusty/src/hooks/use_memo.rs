use crate::hooks::deps::{clone_deps, deps_changed, DynEq};
use crate::hooks::hook_store::MemoEntry;
use crate::views::view::BuildContext;

/// Memoize a computed value with dependency tracking.
///
/// On first call, runs `compute()` and caches the result. On subsequent builds,
/// returns the cached value if `deps` haven't changed; recomputes otherwise.
///
/// Mirrors Ivy-Framework's `UseMemo.cs` pattern.
pub fn use_memo<T, F>(ctx: &mut BuildContext, deps: &[&dyn DynEq], compute: F) -> T
where
    T: Send + Sync + Clone + 'static,
    F: FnOnce() -> T,
{
    let idx = ctx.next_hook_index();

    // Check if we have a cached value with matching deps
    if let Some(entry) = ctx.store.memos.get(&idx) {
        if !deps_changed(&entry.prev_deps, deps) {
            if let Some(cached) = entry.value.downcast_ref::<T>() {
                return cached.clone();
            }
        }
    }

    // Compute new value
    let value = compute();

    // Store in memo cache
    ctx.store.memos.insert(
        idx,
        MemoEntry {
            value: Box::new(value.clone()),
            prev_deps: clone_deps(deps),
        },
    );

    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_use_memo_computes_initially() {
        let mut store = HookStore::new();
        let mut ctx = BuildContext::new(&mut store, None);
        let dep = 1i32;
        let value = use_memo(&mut ctx, &[&dep as &dyn DynEq], || "computed".to_string());
        assert_eq!(value, "computed");
    }

    #[test]
    fn test_use_memo_returns_cached_when_deps_unchanged() {
        let mut store = HookStore::new();
        let compute_count = Arc::new(Mutex::new(0));
        let dep = 42i32;

        // First build
        {
            let cc = compute_count.clone();
            let mut ctx = BuildContext::new(&mut store, None);
            let _ = use_memo(&mut ctx, &[&dep as &dyn DynEq], move || {
                *cc.lock().unwrap() += 1;
                100
            });
        }

        // Second build with same deps
        {
            let cc = compute_count.clone();
            let mut ctx = BuildContext::new(&mut store, None);
            let value = use_memo(&mut ctx, &[&dep as &dyn DynEq], move || {
                *cc.lock().unwrap() += 1;
                999
            });
            assert_eq!(value, 100); // cached
        }

        assert_eq!(*compute_count.lock().unwrap(), 1);
    }

    #[test]
    fn test_use_memo_recomputes_when_deps_change() {
        let mut store = HookStore::new();

        // First build with dep = 1
        {
            let mut ctx = BuildContext::new(&mut store, None);
            let dep = 1i32;
            let value = use_memo(&mut ctx, &[&dep as &dyn DynEq], || dep * 10);
            assert_eq!(value, 10);
        }

        // Second build with dep = 2
        {
            let mut ctx = BuildContext::new(&mut store, None);
            let dep = 2i32;
            let value = use_memo(&mut ctx, &[&dep as &dyn DynEq], || dep * 10);
            assert_eq!(value, 20);
        }
    }
}
