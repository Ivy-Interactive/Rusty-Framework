use std::sync::{Arc, RwLock};

/// Reactive state handle returned by `use_state`.
///
/// `State<T>` is cheaply cloneable (Arc-backed) and can be shared across closures.
#[derive(Debug)]
pub struct State<T: Send + Sync + 'static> {
    inner: Arc<RwLock<T>>,
}

impl<T: Send + Sync + Clone + 'static> State<T> {
    fn new(initial: T) -> Self {
        State {
            inner: Arc::new(RwLock::new(initial)),
        }
    }

    /// Get the current value.
    pub fn get(&self) -> T {
        self.inner.read().unwrap().clone()
    }

    /// Set a new value.
    pub fn set(&self, value: T) {
        let mut guard = self.inner.write().unwrap();
        *guard = value;
    }

    /// Update the value using a function.
    pub fn update(&self, f: impl FnOnce(&T) -> T) {
        let mut guard = self.inner.write().unwrap();
        let new_val = f(&*guard);
        *guard = new_val;
    }
}

impl<T: Send + Sync + Clone + 'static> Clone for State<T> {
    fn clone(&self) -> Self {
        State {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Create a reactive state value. Analogous to React's useState.
///
/// # Example
/// ```
/// use rusty::hooks::use_state;
/// use rusty::views::view::BuildContext;
///
/// let mut ctx = BuildContext::new();
/// let count = use_state(&mut ctx, 0);
/// assert_eq!(count.get(), 0);
/// count.set(1);
/// assert_eq!(count.get(), 1);
/// ```
pub fn use_state<T: Send + Sync + Clone + 'static>(
    ctx: &mut crate::views::view::BuildContext,
    initial: T,
) -> State<T> {
    let _idx = ctx.next_hook_index();
    let state = State::new(initial);
    ctx.store_state(Box::new(state.clone()));
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::view::BuildContext;

    #[test]
    fn test_state_get_set() {
        let mut ctx = BuildContext::new();
        let state = use_state(&mut ctx, 42);
        assert_eq!(state.get(), 42);
        state.set(100);
        assert_eq!(state.get(), 100);
    }

    #[test]
    fn test_state_update() {
        let mut ctx = BuildContext::new();
        let state = use_state(&mut ctx, 10);
        state.update(|v| v + 5);
        assert_eq!(state.get(), 15);
    }

    #[test]
    fn test_state_clone_shares_value() {
        let mut ctx = BuildContext::new();
        let state1 = use_state(&mut ctx, 0);
        let state2 = state1.clone();
        state1.set(99);
        assert_eq!(state2.get(), 99);
    }
}
