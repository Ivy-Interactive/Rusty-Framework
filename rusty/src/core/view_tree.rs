use std::collections::HashMap;
use std::sync::Arc;

use crate::shared::ViewId;
use crate::views::view::View;

/// A node in the view tree, representing a single view with its parent/child relationships.
pub struct ViewNode {
    pub view_id: ViewId,
    pub view: Arc<dyn View>,
    pub parent: Option<ViewId>,
    pub children: Vec<ViewId>,
}

/// A tree of views rooted at a single root node.
///
/// Tracks parent-child relationships between views, enabling:
/// - Ancestor walking (for `use_context`)
/// - Subtree identification (for targeted rebuilds and cleanup)
/// - Independent per-view HookStores
pub struct ViewTree {
    nodes: HashMap<ViewId, ViewNode>,
    root_id: ViewId,
}

impl ViewTree {
    /// Create a new ViewTree with a root view.
    pub fn new(root_view: Box<dyn View>) -> Self {
        let root_id = uuid::Uuid::new_v4();
        let root_node = ViewNode {
            view_id: root_id,
            view: Arc::from(root_view),
            parent: None,
            children: Vec::new(),
        };
        let mut nodes = HashMap::new();
        nodes.insert(root_id, root_node);
        ViewTree { nodes, root_id }
    }

    /// Get the root view ID.
    pub fn root_id(&self) -> ViewId {
        self.root_id
    }

    /// Insert a child view under the given parent, returning the new child's ViewId.
    pub fn insert(&mut self, parent_id: ViewId, view: Box<dyn View>) -> ViewId {
        let child_id = uuid::Uuid::new_v4();
        let child_node = ViewNode {
            view_id: child_id,
            view: Arc::from(view),
            parent: Some(parent_id),
            children: Vec::new(),
        };
        self.nodes.insert(child_id, child_node);
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.push(child_id);
        }
        child_id
    }

    /// Insert a child view with a specific ViewId (for stable keying).
    pub fn insert_with_id(
        &mut self,
        parent_id: ViewId,
        child_id: ViewId,
        view: Box<dyn View>,
    ) -> ViewId {
        let child_node = ViewNode {
            view_id: child_id,
            view: Arc::from(view),
            parent: Some(parent_id),
            children: Vec::new(),
        };
        self.nodes.insert(child_id, child_node);
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            if !parent.children.contains(&child_id) {
                parent.children.push(child_id);
            }
        }
        child_id
    }

    /// Remove a node and all its descendants from the tree.
    /// Returns the list of removed ViewIds (for HookStore cleanup).
    pub fn remove(&mut self, view_id: ViewId) -> Vec<ViewId> {
        let removed_ids = self.subtree_ids(view_id);

        // Unlink from parent
        if let Some(node) = self.nodes.get(&view_id) {
            if let Some(parent_id) = node.parent {
                if let Some(parent) = self.nodes.get_mut(&parent_id) {
                    parent.children.retain(|c| *c != view_id);
                }
            }
        }

        // Remove all nodes in the subtree
        for id in &removed_ids {
            self.nodes.remove(id);
        }

        removed_ids
    }

    /// Iterate over ancestors of a view, from parent up to root (not including self).
    pub fn ancestors(&self, view_id: ViewId) -> Vec<ViewId> {
        let mut result = Vec::new();
        let mut current = view_id;
        while let Some(node) = self.nodes.get(&current) {
            if let Some(parent_id) = node.parent {
                result.push(parent_id);
                current = parent_id;
            } else {
                break;
            }
        }
        result
    }

    /// Get all descendants of a view (including the view itself).
    pub fn subtree_ids(&self, view_id: ViewId) -> Vec<ViewId> {
        let mut result = Vec::new();
        let mut stack = vec![view_id];
        while let Some(id) = stack.pop() {
            result.push(id);
            if let Some(node) = self.nodes.get(&id) {
                for child_id in &node.children {
                    stack.push(*child_id);
                }
            }
        }
        result
    }

    /// Get a reference to a view node by ID.
    pub fn get(&self, view_id: &ViewId) -> Option<&ViewNode> {
        self.nodes.get(view_id)
    }

    /// Get a mutable reference to a view node by ID.
    pub fn get_mut(&mut self, view_id: &ViewId) -> Option<&mut ViewNode> {
        self.nodes.get_mut(view_id)
    }

    /// Get the children of a view.
    pub fn children(&self, view_id: &ViewId) -> Vec<ViewId> {
        self.nodes
            .get(view_id)
            .map(|n| n.children.clone())
            .unwrap_or_default()
    }

    /// Clear the children list for a view (used before rebuilding to detect removed children).
    pub fn clear_children(&mut self, view_id: &ViewId) {
        if let Some(node) = self.nodes.get_mut(view_id) {
            node.children.clear();
        }
    }

    /// Check if a view exists in the tree.
    pub fn contains(&self, view_id: &ViewId) -> bool {
        self.nodes.contains_key(view_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::view::{BuildContext, Element};

    #[allow(dead_code)]
    struct DummyView(String);
    impl View for DummyView {
        fn build(&self, _ctx: &mut BuildContext) -> Element {
            Element::Empty
        }
    }

    #[test]
    fn test_view_tree_insert_and_ancestors() {
        let mut tree = ViewTree::new(Box::new(DummyView("root".into())));
        let root = tree.root_id();

        let child = tree.insert(root, Box::new(DummyView("child".into())));
        let grandchild = tree.insert(child, Box::new(DummyView("grandchild".into())));

        let ancestors = tree.ancestors(grandchild);
        assert_eq!(ancestors.len(), 2);
        assert_eq!(ancestors[0], child);
        assert_eq!(ancestors[1], root);

        let root_ancestors = tree.ancestors(root);
        assert!(root_ancestors.is_empty());
    }

    #[test]
    fn test_view_tree_remove_subtree() {
        let mut tree = ViewTree::new(Box::new(DummyView("root".into())));
        let root = tree.root_id();

        let child = tree.insert(root, Box::new(DummyView("child".into())));
        let gc1 = tree.insert(child, Box::new(DummyView("gc1".into())));
        let gc2 = tree.insert(child, Box::new(DummyView("gc2".into())));

        let removed = tree.remove(child);
        assert_eq!(removed.len(), 3);
        assert!(removed.contains(&child));
        assert!(removed.contains(&gc1));
        assert!(removed.contains(&gc2));

        // Root should have no children now
        assert!(tree.children(&root).is_empty());
        // Removed nodes should not be in the tree
        assert!(!tree.contains(&child));
        assert!(!tree.contains(&gc1));
    }

    #[test]
    fn test_view_tree_subtree_ids() {
        let mut tree = ViewTree::new(Box::new(DummyView("root".into())));
        let root = tree.root_id();

        let c1 = tree.insert(root, Box::new(DummyView("c1".into())));
        let c2 = tree.insert(root, Box::new(DummyView("c2".into())));
        let gc = tree.insert(c1, Box::new(DummyView("gc".into())));

        let subtree = tree.subtree_ids(root);
        assert_eq!(subtree.len(), 4);

        let sub_c1 = tree.subtree_ids(c1);
        assert_eq!(sub_c1.len(), 2);
        assert!(sub_c1.contains(&c1));
        assert!(sub_c1.contains(&gc));

        let sub_c2 = tree.subtree_ids(c2);
        assert_eq!(sub_c2.len(), 1);
        assert!(sub_c2.contains(&c2));
    }
}
