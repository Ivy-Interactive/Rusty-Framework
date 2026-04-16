use crate::core::diff::{diff, Patch};
use serde_json::Value;

/// The reconciler compares old and new widget trees and produces patches.
pub struct Reconciler {
    previous_tree: Option<Value>,
}

impl Reconciler {
    pub fn new() -> Self {
        Reconciler {
            previous_tree: None,
        }
    }

    /// Reconcile a new tree against the previously stored tree.
    /// Returns patches if there was a previous tree, or None for the initial render.
    pub fn reconcile(&mut self, new_tree: &Value) -> Option<Vec<Patch>> {
        let patches = self.previous_tree.as_ref().map(|old| diff(old, new_tree));
        self.previous_tree = Some(new_tree.clone());
        patches
    }

    /// Check if this is the first render (no previous tree).
    pub fn is_initial(&self) -> bool {
        self.previous_tree.is_none()
    }
}

impl Default for Reconciler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_reconciler_initial_render() {
        let mut reconciler = Reconciler::new();
        assert!(reconciler.is_initial());

        let tree = json!({"type": "text", "content": "hello"});
        let patches = reconciler.reconcile(&tree);
        assert!(patches.is_none()); // First render, no patches
        assert!(!reconciler.is_initial());
    }

    #[test]
    fn test_reconciler_subsequent_render() {
        let mut reconciler = Reconciler::new();
        let tree1 = json!({"type": "text", "content": "hello"});
        reconciler.reconcile(&tree1);

        let tree2 = json!({"type": "text", "content": "world"});
        let patches = reconciler.reconcile(&tree2);
        assert!(patches.is_some());
        assert!(!patches.unwrap().is_empty());
    }
}
