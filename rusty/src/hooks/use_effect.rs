use crate::views::view::BuildContext;

/// Register a side effect that runs after the view builds.
///
/// The effect callback is registered and will be executed by the runtime
/// after the build phase completes.
///
/// # Example
/// ```
/// use rusty::hooks::use_effect;
/// use rusty::views::view::BuildContext;
///
/// let mut ctx = BuildContext::new();
/// use_effect(&mut ctx, || {
///     println!("Effect ran!");
/// });
/// ```
pub fn use_effect<F>(ctx: &mut BuildContext, callback: F)
where
    F: FnOnce() + Send + 'static,
{
    let _idx = ctx.next_hook_index();
    ctx.register_effect(Box::new(callback));
}

/// Register a side effect with a dependency check.
///
/// The effect only runs if `deps_changed` is true, allowing callers to
/// implement their own dependency comparison logic.
pub fn use_effect_with_deps<F>(ctx: &mut BuildContext, deps_changed: bool, callback: F)
where
    F: FnOnce() + Send + 'static,
{
    let _idx = ctx.next_hook_index();
    if deps_changed {
        ctx.register_effect(Box::new(callback));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_use_effect_registers() {
        let mut ctx = BuildContext::new();
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();

        use_effect(&mut ctx, move || {
            *called_clone.lock().unwrap() = true;
        });

        // Drain and execute effects
        let effects = ctx.drain_effects();
        assert_eq!(effects.len(), 1);
        for effect in effects {
            effect();
        }
        assert!(*called.lock().unwrap());
    }

    #[test]
    fn test_use_effect_with_deps_skips_when_unchanged() {
        let mut ctx = BuildContext::new();

        use_effect_with_deps(&mut ctx, false, || {
            panic!("Should not be called");
        });

        let effects = ctx.drain_effects();
        assert_eq!(effects.len(), 0);
    }
}
