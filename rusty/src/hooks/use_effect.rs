use crate::hooks::deps::{clone_deps, deps_changed, DynEq};
use crate::hooks::hook_store::EffectEntry;
use crate::views::view::BuildContext;

/// Register a mount-only side effect (runs once on first build).
///
/// The callback can optionally return a cleanup function that will be called
/// before the effect re-runs or when the view unmounts.
pub fn use_effect<F>(ctx: &mut BuildContext, callback: F)
where
    F: FnOnce() -> Option<Box<dyn FnOnce() + Send + Sync>> + Send + 'static,
{
    let idx = ctx.next_hook_index();

    // Check if this effect has already run (mount-only)
    let has_run = ctx
        .store
        .effects
        .get(&idx)
        .map(|e| e.has_run)
        .unwrap_or(false);

    if !has_run {
        // Initialize the effect entry and mark as run
        ctx.store.effects.entry(idx).or_insert_with(|| EffectEntry {
            prev_deps: None,
            cleanup: None,
            has_run: true,
        });
        if let Some(entry) = ctx.store.effects.get_mut(&idx) {
            entry.has_run = true;
        }

        ctx.register_effect(idx, Box::new(callback));
    }
}

/// Register a side effect that fires when dependencies change.
///
/// Analogous to Ivy-Framework's trigger-based effect system — the effect runs
/// on first build and again whenever any dependency value changes.
pub fn use_effect_with_deps<F>(ctx: &mut BuildContext, deps: &[&dyn DynEq], callback: F)
where
    F: FnOnce(&[&dyn DynEq]) -> Option<Box<dyn FnOnce() + Send + Sync>> + Send + 'static,
{
    let idx = ctx.next_hook_index();

    let should_run = if let Some(entry) = ctx.store.effects.get(&idx) {
        if let Some(ref prev) = entry.prev_deps {
            deps_changed(prev, deps)
        } else {
            true
        }
    } else {
        true
    };

    if should_run {
        let owned_deps = clone_deps(deps);

        // Update stored deps
        let entry = ctx.store.effects.entry(idx).or_insert_with(|| EffectEntry {
            prev_deps: None,
            cleanup: None,
            has_run: false,
        });
        entry.prev_deps = Some(clone_deps(deps));
        entry.has_run = true;

        ctx.register_effect(
            idx,
            Box::new(move || {
                let dep_refs: Vec<&dyn DynEq> = owned_deps
                    .iter()
                    .map(|b| b.as_ref() as &dyn DynEq)
                    .collect();
                callback(&dep_refs)
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_use_effect_runs_on_first_build() {
        let mut store = HookStore::new();
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();

        {
            let mut ctx = BuildContext::new(&mut store, None);
            use_effect(&mut ctx, move || {
                *called_clone.lock().unwrap() = true;
                None
            });

            let effects = ctx.drain_effects();
            assert_eq!(effects.len(), 1);
            for e in effects {
                (e.callback)();
            }
        }

        assert!(*called.lock().unwrap());
    }

    #[test]
    fn test_use_effect_mount_only_skips_subsequent_builds() {
        let mut store = HookStore::new();
        let call_count = Arc::new(Mutex::new(0));

        // First build
        {
            let cc = call_count.clone();
            let mut ctx = BuildContext::new(&mut store, None);
            use_effect(&mut ctx, move || {
                *cc.lock().unwrap() += 1;
                None
            });
            let effects = ctx.drain_effects();
            for e in effects {
                (e.callback)();
            }
        }

        // Second build — should NOT fire
        {
            let cc = call_count.clone();
            let mut ctx = BuildContext::new(&mut store, None);
            use_effect(&mut ctx, move || {
                *cc.lock().unwrap() += 1;
                None
            });
            let effects = ctx.drain_effects();
            assert_eq!(effects.len(), 0);
        }

        assert_eq!(*call_count.lock().unwrap(), 1);
    }

    #[test]
    fn test_use_effect_cleanup_called() {
        let mut store = HookStore::new();
        let cleaned_up = Arc::new(Mutex::new(false));
        let cleaned_up_clone = cleaned_up.clone();

        // First build — register effect with cleanup
        {
            let mut ctx = BuildContext::new(&mut store, None);
            let dep = 1i32;
            use_effect_with_deps(&mut ctx, &[&dep as &dyn DynEq], move |_| {
                Some(Box::new(move || {
                    *cleaned_up_clone.lock().unwrap() = true;
                }) as Box<dyn FnOnce() + Send + Sync>)
            });
            let effects = ctx.drain_effects();
            for e in effects {
                let cleanup = (e.callback)();
                if let Some(entry) = store.effects.get_mut(&e.hook_index) {
                    entry.cleanup = cleanup;
                }
            }
        }

        // Second build with changed deps — cleanup should be called
        {
            let mut ctx = BuildContext::new(&mut store, None);
            let dep = 2i32;
            use_effect_with_deps(&mut ctx, &[&dep as &dyn DynEq], |_| None);
            let effects = ctx.drain_effects();
            for e in effects {
                if let Some(entry) = store.effects.get_mut(&e.hook_index) {
                    if let Some(cleanup) = entry.cleanup.take() {
                        cleanup();
                    }
                }
                (e.callback)();
            }
        }

        assert!(*cleaned_up.lock().unwrap());
    }

    #[test]
    fn test_use_effect_with_deps_skips_when_unchanged() {
        let mut store = HookStore::new();
        let call_count = Arc::new(Mutex::new(0));
        let dep = 42i32;

        // First build
        {
            let cc = call_count.clone();
            let mut ctx = BuildContext::new(&mut store, None);
            use_effect_with_deps(&mut ctx, &[&dep as &dyn DynEq], move |_| {
                *cc.lock().unwrap() += 1;
                None
            });
            let effects = ctx.drain_effects();
            for e in effects {
                (e.callback)();
            }
        }

        // Second build with same deps — should skip
        {
            let cc = call_count.clone();
            let mut ctx = BuildContext::new(&mut store, None);
            use_effect_with_deps(&mut ctx, &[&dep as &dyn DynEq], move |_| {
                *cc.lock().unwrap() += 1;
                None
            });
            let effects = ctx.drain_effects();
            assert_eq!(effects.len(), 0);
        }

        assert_eq!(*call_count.lock().unwrap(), 1);
    }

    #[test]
    fn test_use_effect_with_deps_fires_on_change() {
        let mut store = HookStore::new();
        let call_count = Arc::new(Mutex::new(0));

        // First build with dep = 1
        {
            let cc = call_count.clone();
            let mut ctx = BuildContext::new(&mut store, None);
            let dep = 1i32;
            use_effect_with_deps(&mut ctx, &[&dep as &dyn DynEq], move |_| {
                *cc.lock().unwrap() += 1;
                None
            });
            let effects = ctx.drain_effects();
            for e in effects {
                (e.callback)();
            }
        }

        // Second build with dep = 2 — should fire
        {
            let cc = call_count.clone();
            let mut ctx = BuildContext::new(&mut store, None);
            let dep = 2i32;
            use_effect_with_deps(&mut ctx, &[&dep as &dyn DynEq], move |_| {
                *cc.lock().unwrap() += 1;
                None
            });
            let effects = ctx.drain_effects();
            assert_eq!(effects.len(), 1);
            for e in effects {
                (e.callback)();
            }
        }

        assert_eq!(*call_count.lock().unwrap(), 2);
    }
}
