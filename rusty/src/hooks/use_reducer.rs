use crate::hooks::use_state::{use_state, State};
use crate::views::view::BuildContext;
use std::sync::Arc;

/// Create a reducer-based state with a dispatch function.
///
/// Ported from Ivy-Framework's `UseReducer.cs`. Built on `use_state` internally —
/// the reducer function is applied on each dispatch to produce a new state.
///
/// Returns `(state, dispatch)` where `dispatch` accepts an action and updates
/// the state by applying `reducer(current_state, action)`.
pub fn use_reducer<T, A>(
    ctx: &mut BuildContext,
    reducer: fn(&T, A) -> T,
    initial: T,
) -> (State<T>, Arc<dyn Fn(A) + Send + Sync>)
where
    T: Send + Sync + Clone + 'static,
    A: Send + Sync + 'static,
{
    let state = use_state(ctx, initial);
    let state_clone = state.clone();

    let dispatch = Arc::new(move |action: A| {
        let current = state_clone.get();
        let new_val = reducer(&current, action);
        state_clone.set(new_val);
    }) as Arc<dyn Fn(A) + Send + Sync>;

    (state, dispatch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;

    #[derive(Debug, Clone)]
    enum CountAction {
        Increment,
        Decrement,
        Add(i32),
    }

    fn count_reducer(state: &i32, action: CountAction) -> i32 {
        match action {
            CountAction::Increment => state + 1,
            CountAction::Decrement => state - 1,
            CountAction::Add(n) => state + n,
        }
    }

    #[test]
    fn test_use_reducer_basic() {
        let mut store = HookStore::new();
        let mut ctx = BuildContext::new(&mut store, None);
        let (state, dispatch) = use_reducer(&mut ctx, count_reducer, 0);

        assert_eq!(state.get(), 0);
        dispatch(CountAction::Increment);
        assert_eq!(state.get(), 1);
        dispatch(CountAction::Add(10));
        assert_eq!(state.get(), 11);
        dispatch(CountAction::Decrement);
        assert_eq!(state.get(), 10);
    }

    #[test]
    fn test_use_reducer_persists_across_builds() {
        let mut store = HookStore::new();

        // First build
        {
            let mut ctx = BuildContext::new(&mut store, None);
            let (_, dispatch) = use_reducer(&mut ctx, count_reducer, 0);
            dispatch(CountAction::Add(5));
        }

        // Second build — state preserved
        {
            let mut ctx = BuildContext::new(&mut store, None);
            let (state, _) = use_reducer(&mut ctx, count_reducer, 0);
            assert_eq!(state.get(), 5);
        }
    }
}
