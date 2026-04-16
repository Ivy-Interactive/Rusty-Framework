use std::sync::{Arc, RwLock};

use crate::shared::ViewId;
use crate::views::view::BuildContext;

/// Reactive state handle returned by `use_state`.
///
/// `State<T>` is cheaply cloneable (Arc-backed) and can be shared across closures.
/// When `set()` or `update()` is called, it sends a rebuild signal to the runtime's
/// event loop so the view re-renders automatically. The signal carries the owning
/// ViewId so the runtime can do targeted subtree rebuilds.
#[derive(Debug)]
pub struct State<T: Send + Sync + 'static> {
    inner: Arc<RwLock<T>>,
    rebuild_tx: Option<tokio::sync::mpsc::Sender<ViewId>>,
    /// The ViewId that owns this state, sent with rebuild signals.
    view_id: ViewId,
    /// When true, mutations do NOT trigger rebuilds (used by use_ref).
    pub(crate) silent: bool,
}

impl<T: Send + Sync + Clone + 'static> State<T> {
    pub(crate) fn new(
        initial: T,
        rebuild_tx: Option<tokio::sync::mpsc::Sender<ViewId>>,
        view_id: ViewId,
    ) -> Self {
        State {
            inner: Arc::new(RwLock::new(initial)),
            rebuild_tx,
            view_id,
            silent: false,
        }
    }

    /// Create a silent state that doesn't trigger rebuilds on mutation.
    pub(crate) fn new_silent(initial: T) -> Self {
        State {
            inner: Arc::new(RwLock::new(initial)),
            rebuild_tx: None,
            view_id: uuid::Uuid::nil(),
            silent: true,
        }
    }

    /// Get the current value.
    pub fn get(&self) -> T {
        self.inner.read().unwrap().clone()
    }

    /// Set a new value and trigger a rebuild (unless silent).
    pub fn set(&self, value: T) {
        {
            let mut guard = self.inner.write().unwrap();
            *guard = value;
        }
        self.notify_rebuild();
    }

    /// Update the value using a function and trigger a rebuild (unless silent).
    pub fn update(&self, f: impl FnOnce(&T) -> T) {
        {
            let mut guard = self.inner.write().unwrap();
            let new_val = f(&*guard);
            *guard = new_val;
        }
        self.notify_rebuild();
    }

    fn notify_rebuild(&self) {
        if self.silent {
            return;
        }
        if let Some(ref tx) = self.rebuild_tx {
            let _ = tx.try_send(self.view_id);
        }
    }
}

impl<T: Send + Sync + Clone + 'static> Clone for State<T> {
    fn clone(&self) -> Self {
        State {
            inner: Arc::clone(&self.inner),
            rebuild_tx: self.rebuild_tx.clone(),
            view_id: self.view_id,
            silent: self.silent,
        }
    }
}

/// Create a reactive state value with persistence across re-renders.
///
/// On first call, initializes with `initial`. On subsequent builds, returns
/// the persisted state from the HookStore (mutations are preserved).
pub fn use_state<T: Send + Sync + Clone + 'static>(ctx: &mut BuildContext, initial: T) -> State<T> {
    let idx = ctx.next_hook_index();
    let rebuild_info = ctx.rebuild_sender();
    let (rebuild_tx, view_id) = match rebuild_info {
        Some((tx, vid)) => (Some(tx), vid),
        None => (None, uuid::Uuid::nil()),
    };
    ctx.store
        .get_or_init_state(idx, || State::new(initial, rebuild_tx, view_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;

    #[test]
    fn test_state_get_set() {
        let mut store = HookStore::new();
        let mut ctx = BuildContext::new(&mut store, None);
        let state = use_state(&mut ctx, 42);
        assert_eq!(state.get(), 42);
        state.set(100);
        assert_eq!(state.get(), 100);
    }

    #[test]
    fn test_state_update() {
        let mut store = HookStore::new();
        let mut ctx = BuildContext::new(&mut store, None);
        let state = use_state(&mut ctx, 10);
        state.update(|v| v + 5);
        assert_eq!(state.get(), 15);
    }

    #[test]
    fn test_state_clone_shares_value() {
        let mut store = HookStore::new();
        let mut ctx = BuildContext::new(&mut store, None);
        let state1 = use_state(&mut ctx, 0);
        let state2 = state1.clone();
        state1.set(99);
        assert_eq!(state2.get(), 99);
    }

    #[test]
    fn test_state_persists_across_builds() {
        let mut store = HookStore::new();

        // First build
        {
            let mut ctx = BuildContext::new(&mut store, None);
            let state = use_state(&mut ctx, 0);
            state.set(42);
        }

        // Second build — state should be preserved
        {
            let mut ctx = BuildContext::new(&mut store, None);
            let state = use_state::<i32>(&mut ctx, 0);
            assert_eq!(state.get(), 42);
        }
    }

    #[test]
    fn test_state_set_triggers_rebuild() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let mut store = HookStore::new();
        let view_id = uuid::Uuid::new_v4();
        let mut ctx = BuildContext::with_view_id(&mut store, Some(tx), view_id);
        let state = use_state(&mut ctx, 0);

        state.set(1);
        let received = rx.try_recv().unwrap();
        assert_eq!(received, view_id);
    }
}
