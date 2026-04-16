use crate::views::view::BuildContext;

/// Memoize a computed value. The compute function runs on every call for now;
/// a full implementation would cache based on dependency comparison.
///
/// # Example
/// ```
/// use rusty::hooks::use_memo;
/// use rusty::views::view::BuildContext;
///
/// let mut ctx = BuildContext::new();
/// let doubled = use_memo(&mut ctx, || 21 * 2);
/// assert_eq!(doubled, 42);
/// ```
pub fn use_memo<T, F>(ctx: &mut BuildContext, compute: F) -> T
where
    T: Send + Sync + Clone + 'static,
    F: FnOnce() -> T,
{
    let _idx = ctx.next_hook_index();
    compute()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_memo() {
        let mut ctx = BuildContext::new();
        let value = use_memo(&mut ctx, || "computed".to_string());
        assert_eq!(value, "computed");
    }
}
