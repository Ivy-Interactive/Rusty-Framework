use std::any::Any;
use std::collections::HashMap;

use super::deps::DynEq;
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
    /// Context values keyed by TypeId name (for use_context).
    pub contexts: HashMap<std::any::TypeId, Box<dyn Any + Send + Sync>>,
}

impl HookStore {
    pub fn new() -> Self {
        HookStore {
            states: Vec::new(),
            effects: HashMap::new(),
            memos: HashMap::new(),
            contexts: HashMap::new(),
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
    pub fn cleanup_all_effects(&mut self) {
        for (_, entry) in self.effects.drain() {
            if let Some(cleanup) = entry.cleanup {
                cleanup();
            }
        }
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
        assert_eq!(
            store.get_or_init_state::<String>(1, || String::new()),
            "hello"
        );
        assert_eq!(store.get_or_init_state::<bool>(2, || false), true);
    }

    #[test]
    fn test_update_state() {
        let mut store = HookStore::new();
        let _: i32 = store.get_or_init_state(0, || 42);
        store.update_state(0, 100i32);
        assert_eq!(store.get_or_init_state::<i32>(0, || 0), 100);
    }
}
