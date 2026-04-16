use crate::views::view::BuildContext;
use std::sync::Arc;

/// Create a stable callback reference. Wraps the closure in an Arc for
/// cheap cloning across widget boundaries.
///
/// # Example
/// ```
/// use rusty::hooks::use_callback;
/// use rusty::views::view::BuildContext;
///
/// let mut ctx = BuildContext::new();
/// let on_click = use_callback(&mut ctx, |_: ()| {
///     println!("Clicked!");
/// });
/// on_click(());
/// ```
pub fn use_callback<F, A>(ctx: &mut BuildContext, callback: F) -> Arc<dyn Fn(A) + Send + Sync>
where
    F: Fn(A) + Send + Sync + 'static,
    A: 'static,
{
    let _idx = ctx.next_hook_index();
    Arc::new(callback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn test_use_callback() {
        let mut ctx = BuildContext::new();
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        let cb = use_callback(&mut ctx, move |_: ()| {
            *counter_clone.lock().unwrap() += 1;
        });

        cb(());
        cb(());
        assert_eq!(*counter.lock().unwrap(), 2);
    }
}
