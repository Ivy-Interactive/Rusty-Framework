use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use super::deps::DynEq;
use crate::shared::ViewId;
use crate::views::view::EffectCleanup;

/// Entry for a stored effect in the HookStore.
pub struct EffectEntry {
    /// Previous dependency values for comparison.
    pub prev_deps: Option<Vec<Box<dyn DynEq>>>,
    /// Cleanup function from the last effect execution.
    pub cleanup: Option<EffectCleanup>,
    /// Whether this effect has run at least once (for mount detection).
    pub has_run: bool,
}

/// Entry for a stored memo value in the HookStore.
pub struct MemoEntry {
    /// The cached computed value (type-erased).
    pub value: Box<dyn Any + Send + Sync>,
    /// Previous dependency values for comparison.
    pub prev_deps: Vec<Box<dyn DynEq>>,
}

/// Persistent hook state store that survives across re-renders.
///
/// Analogous to Ivy-Framework's `ViewContext._hooks` and `_effects` dictionaries.
/// Each slot is keyed by hook call index (same ordering rule as React/Ivy).
pub struct HookStore {
    /// Persisted state slots (keyed by hook call index via Vec position).
    pub states: Vec<Option<Box<dyn Any + Send + Sync>>>,
    /// Persisted effect entries keyed by hook index.
    pub effects: HashMap<usize, EffectEntry>,
    /// Cached memo values keyed by hook index.
    pub memos: HashMap<usize, MemoEntry>,
    /// Context values keyed by TypeId (for use_context). Uses `Arc` so ancestor
    /// context snapshots can be cheaply cloned without raw pointers.
    pub contexts: HashMap<std::any::TypeId, Arc<dyn Any + Send + Sync>>,
    /// Persisted stores for child views embedded via `child_view()`, keyed by the
    /// child's deterministic ViewId. Nesting the child's store inside the parent's
    /// keys effect cleanups by `(view_id, hook_index)` instead of `hook_index` alone.
    pub child_stores: HashMap<ViewId, HookStore>,
}

impl HookStore {
    pub fn new() -> Self {
        HookStore {
            states: Vec::new(),
            effects: HashMap::new(),
            memos: HashMap::new(),
            contexts: HashMap::new(),
            child_stores: HashMap::new(),
        }
    }

    /// Get or initialize a state slot at the given index.
    /// If the slot exists and contains a value of type T, returns it.
    /// Otherwise, initializes with the provided function.
    pub fn get_or_init_state<T: Send + Sync + Clone + 'static>(
        &mut self,
        index: usize,
        init: impl FnOnce() -> T,
    ) -> T {
        // Ensure the vec is long enough
        while self.states.len() <= index {
            self.states.push(None);
        }

        if let Some(ref existing) = self.states[index] {
            if let Some(val) = existing.downcast_ref::<T>() {
                return val.clone();
            }
        }

        // Initialize
        let val = init();
        self.states[index] = Some(Box::new(val.clone()));
        val
    }

    /// Update the stored state value at the given index.
    pub fn update_state<T: Send + Sync + Clone + 'static>(&mut self, index: usize, value: T) {
        while self.states.len() <= index {
            self.states.push(None);
        }
        self.states[index] = Some(Box::new(value));
    }

    /// Run all effect cleanups (called on unmount).
    ///
    /// Recurses into `child_stores` — a parent unmount must also tear down every
    /// descendant embedded via `child_view()`, or their cleanups leak.
    pub fn cleanup_all_effects(&mut self) {
        for (_, entry) in self.effects.drain() {
            if let Some(cleanup) = entry.cleanup {
                cleanup();
            }
        }
        for (_, mut child) in self.child_stores.drain() {
            child.cleanup_all_effects();
        }
    }

    /// Resolve the store owning `view_id`, descending through nested child stores.
    /// `self_id` is the ViewId this store belongs to. `None` means the id is
    /// neither this store's view nor any descendant's.
    pub fn store_for_mut(&mut self, self_id: ViewId, view_id: ViewId) -> Option<&mut HookStore> {
        if self_id == view_id {
            return Some(self);
        }
        for (child_id, child) in self.child_stores.iter_mut() {
            let cid = *child_id;
            if let Some(found) = child.store_for_mut(cid, view_id) {
                return Some(found);
            }
        }
        None
    }
}

impl Default for HookStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn test_get_or_init_state_initializes() {
        let mut store = HookStore::new();
        let val: i32 = store.get_or_init_state(0, || 42);
        assert_eq!(val, 42);
    }

    #[test]
    fn test_get_or_init_state_returns_existing() {
        let mut store = HookStore::new();
        let _: i32 = store.get_or_init_state(0, || 42);
        // Second call should return the stored value, not re-initialize
        let val: i32 = store.get_or_init_state(0, || 999);
        assert_eq!(val, 42);
    }

    #[test]
    fn test_state_persistence_across_multiple_slots() {
        let mut store = HookStore::new();
        let _: i32 = store.get_or_init_state(0, || 10);
        let _: String = store.get_or_init_state(1, || "hello".to_string());
        let _: bool = store.get_or_init_state(2, || true);

        assert_eq!(store.get_or_init_state::<i32>(0, || 0), 10);
        assert_eq!(store.get_or_init_state::<String>(1, String::new), "hello");
        assert!(store.get_or_init_state::<bool>(2, || false));
    }

    #[test]
    fn test_update_state() {
        let mut store = HookStore::new();
        let _: i32 = store.get_or_init_state(0, || 42);
        store.update_state(0, 100i32);
        assert_eq!(store.get_or_init_state::<i32>(0, || 0), 100);
    }

    /// Inserts an effect entry at `index` whose cleanup pushes `label` onto `log`.
    fn insert_cleanup(
        store: &mut HookStore,
        index: usize,
        log: &Arc<Mutex<Vec<&'static str>>>,
        label: &'static str,
    ) {
        let log = log.clone();
        store.effects.insert(
            index,
            EffectEntry {
                prev_deps: None,
                cleanup: Some(Box::new(move || log.lock().unwrap().push(label))),
                has_run: true,
            },
        );
    }

    #[test]
    fn test_cleanup_all_effects_recurses_into_child_stores() {
        let log: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));

        let mut grandchild = HookStore::new();
        insert_cleanup(&mut grandchild, 0, &log, "grandchild");

        let mut child = HookStore::new();
        insert_cleanup(&mut child, 0, &log, "child");
        child.child_stores.insert(ViewId::new_v4(), grandchild);

        let mut parent = HookStore::new();
        insert_cleanup(&mut parent, 0, &log, "parent");
        parent.child_stores.insert(ViewId::new_v4(), child);

        parent.cleanup_all_effects();

        // Every level must be torn down; order across siblings is HashMap order.
        let mut ran = log.lock().unwrap().clone();
        ran.sort_unstable();
        assert_eq!(ran, vec!["child", "grandchild", "parent"]);

        // The stores are drained, so a second teardown is a no-op.
        assert!(parent.effects.is_empty());
        assert!(parent.child_stores.is_empty());
    }
}
