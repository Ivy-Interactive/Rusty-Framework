use crate::hooks::hook_store::EffectEntry;
use crate::views::view::BuildContext;
use std::sync::Arc;
use std::time::Duration;

/// Set up a recurring interval that fires `callback` at the specified duration.
///
/// Ported from Ivy-Framework's `UseInterval.cs`. Uses `tokio::time::interval`
/// internally with auto-cleanup on unmount via the effect cleanup system.
///
/// Pass `None` for `duration` to pause the interval (the previous interval is
/// cleaned up and no new one is started).
pub fn use_interval<F>(ctx: &mut BuildContext, duration: Option<Duration>, callback: F)
where
    F: Fn() + Send + Sync + 'static,
{
    let idx = ctx.next_hook_index();
    let rebuild_info = ctx.rebuild_sender();
    let (rebuild_tx, view_id) = match rebuild_info {
        Some((tx, vid)) => (Some(tx), vid),
        None => (None, crate::shared::ViewId::nil()),
    };

    // Always register/re-register the interval effect
    let entry = ctx.store.effects.entry(idx).or_insert_with(|| EffectEntry {
        prev_deps: None,
        cleanup: None,
        has_run: false,
    });
    entry.has_run = true;

    let callback = Arc::new(callback);

    ctx.register_effect(
        idx,
        Box::new(move || {
            if let Some(dur) = duration {
                let callback = callback.clone();
                let handle = tokio::spawn(async move {
                    let mut interval = tokio::time::interval(dur);
                    interval.tick().await; // first tick is immediate, skip it
                    loop {
                        interval.tick().await;
                        callback();
                        // Optionally trigger rebuild with owning ViewId
                        if let Some(ref tx) = rebuild_tx {
                            let _ = tx.try_send(view_id);
                        }
                    }
                });

                // Return cleanup that aborts the spawned task
                Some(Box::new(move || {
                    handle.abort();
                }) as Box<dyn FnOnce() + Send + Sync>)
            } else {
                None // paused — no interval, no cleanup needed
            }
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use std::sync::Mutex;

    #[tokio::test]
    async fn test_use_interval_fires_callback() {
        tokio::time::pause();

        let mut store = HookStore::new();
        let call_count = Arc::new(Mutex::new(0));

        // Build — register interval
        let effects = {
            let cc = call_count.clone();
            let mut ctx = BuildContext::new(&mut store, None);
            use_interval(&mut ctx, Some(Duration::from_millis(100)), move || {
                *cc.lock().unwrap() += 1;
            });
            ctx.drain_effects()
        };

        // Execute effects (spawns the interval task)
        let mut cleanups: Vec<Box<dyn FnOnce() + Send + Sync>> = Vec::new();
        for e in effects {
            if let Some(cleanup) = (e.callback)() {
                cleanups.push(cleanup);
            }
        }

        // Advance time step-by-step to give spawned tasks a chance to run
        for _ in 0..4 {
            tokio::time::advance(Duration::from_millis(100)).await;
            // Yield multiple times to let the spawned interval task run
            for _ in 0..5 {
                tokio::task::yield_now().await;
            }
        }

        let count = *call_count.lock().unwrap();
        assert!(count >= 2, "Expected at least 2 calls, got {}", count);

        // Cleanup
        for cleanup in cleanups {
            cleanup();
        }
    }

    #[tokio::test]
    async fn test_use_interval_none_duration_no_fire() {
        let mut store = HookStore::new();

        let effects = {
            let mut ctx = BuildContext::new(&mut store, None);
            use_interval(&mut ctx, None, || {
                panic!("Should not fire with None duration");
            });
            ctx.drain_effects()
        };

        // Execute effects
        for e in effects {
            let cleanup = (e.callback)();
            assert!(cleanup.is_none());
        }
    }
}
