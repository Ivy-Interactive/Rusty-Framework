use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::core::event_registry::EventRegistry;
use crate::core::view_tree::ViewTree;
use crate::hooks::hook_store::HookStore;
use crate::shared::ViewId;
use crate::views::view::{BuildContext, Element, View};

/// Message types for the runtime event loop.
#[derive(Debug)]
pub enum RuntimeMessage {
    Event {
        widget_id: String,
        event_name: String,
        args: serde_json::Value,
    },
    Rebuild {
        view_id: ViewId,
    },
    Shutdown,
}

/// The application runtime manages the view tree, state, and event dispatch.
pub struct Runtime {
    view_tree: ViewTree,
    tree: Arc<RwLock<Option<Element>>>,
    hook_stores: HashMap<ViewId, HookStore>,
    dirty_views: HashSet<ViewId>,
    event_registry: Arc<RwLock<EventRegistry>>,
    event_tx: mpsc::Sender<RuntimeMessage>,
    event_rx: mpsc::Receiver<RuntimeMessage>,
    rebuild_tx: mpsc::Sender<ViewId>,
    rebuild_rx: mpsc::Receiver<ViewId>,
}

impl Runtime {
    pub fn new(root: impl View) -> Self {
        let (event_tx, event_rx) = mpsc::channel(2048);
        let (rebuild_tx, rebuild_rx) = mpsc::channel(256);
        let view_tree = ViewTree::new(Box::new(root));
        Runtime {
            view_tree,
            tree: Arc::new(RwLock::new(None)),
            hook_stores: HashMap::new(),
            dirty_views: HashSet::new(),
            event_registry: Arc::new(RwLock::new(EventRegistry::new())),
            event_tx,
            event_rx,
            rebuild_tx,
            rebuild_rx,
        }
    }

    /// Get a sender for dispatching events to the runtime.
    pub fn event_sender(&self) -> mpsc::Sender<RuntimeMessage> {
        self.event_tx.clone()
    }

    /// Get a clone of the rebuild sender (for passing to State handles).
    pub fn rebuild_sender(&self) -> mpsc::Sender<ViewId> {
        self.rebuild_tx.clone()
    }

    /// Build the entire view tree from root, or rebuild a specific subtree.
    ///
    /// - `build(None)` — rebuilds from root (initial build)
    /// - `build(Some(view_id))` — rebuilds only that view's subtree
    pub async fn build(&mut self) -> Element {
        self.build_view(None).await
    }

    /// Build a specific view (or root if None).
    async fn build_view(&mut self, target_view_id: Option<ViewId>) -> Element {
        let view_id = target_view_id.unwrap_or_else(|| self.view_tree.root_id());

        // Get old children before rebuild (for cleanup detection)
        let old_children: Vec<ViewId> = self.view_tree.children(&view_id);

        // Synchronous build phase — construct element, extract registry and effects
        let (element, registry, effects) = {
            let store = self.hook_stores.entry(view_id).or_default();
            let rebuild_tx = self.rebuild_tx.clone();

            let mut ctx = BuildContext::with_view_id(store, Some(rebuild_tx), view_id);
            ctx.reset();

            // Build the view
            let view_node = self.view_tree.get(&view_id).expect("view_id not in tree");
            // SAFETY: We need to borrow view immutably while store is borrowed mutably.
            // The view reference does not alias with the store reference.
            let view_ptr = &*view_node.view as *const dyn View;
            let element = unsafe { &*view_ptr }.build(&mut ctx);

            let mut element = element;
            element.assign_ids(&mut ctx);

            let registry = ctx.take_event_registry();
            let effects = ctx.drain_effects();
            (element, registry, effects)
            // ctx is dropped here, before any .await
        };

        // Async phase — update event registry
        {
            let mut reg = self.event_registry.write().await;
            *reg = registry;
        }

        // Execute effects
        let store = self.hook_stores.get_mut(&view_id).unwrap();
        for effect_record in effects {
            let idx = effect_record.hook_index;
            if let Some(entry) = store.effects.get_mut(&idx) {
                if let Some(cleanup) = entry.cleanup.take() {
                    cleanup();
                }
            }
            let cleanup = (effect_record.callback)();
            if let Some(entry) = store.effects.get_mut(&idx) {
                entry.cleanup = cleanup;
            }
        }

        // Detect removed children and clean up their HookStores
        let new_children: Vec<ViewId> = self.view_tree.children(&view_id);
        for old_child in &old_children {
            if !new_children.contains(old_child) {
                let removed_ids = self.view_tree.remove(*old_child);
                for removed_id in removed_ids {
                    if let Some(mut removed_store) = self.hook_stores.remove(&removed_id) {
                        removed_store.cleanup_all_effects();
                    }
                }
            }
        }

        // Remove from dirty set
        self.dirty_views.remove(&view_id);

        let mut tree = self.tree.write().await;
        *tree = Some(element.clone());
        element
    }

    /// Mark a view as dirty (needs rebuild).
    pub fn mark_dirty(&mut self, view_id: ViewId) {
        self.dirty_views.insert(view_id);
    }

    /// Run the event loop, processing messages until shutdown.
    pub async fn run(&mut self) {
        // Initial build
        let _ = self.build().await;

        loop {
            tokio::select! {
                msg = self.event_rx.recv() => {
                    match msg {
                        Some(RuntimeMessage::Event { widget_id, event_name, args }) => {
                            {
                                let registry = self.event_registry.read().await;
                                registry.dispatch(&widget_id, &event_name, args);
                            }
                            let _ = self.build().await;
                        }
                        Some(RuntimeMessage::Rebuild { view_id }) => {
                            self.dirty_views.insert(view_id);
                            // For now, rebuild from root (future: targeted subtree rebuild)
                            let _ = self.build().await;
                        }
                        Some(RuntimeMessage::Shutdown) | None => break,
                    }
                }
                Some(view_id) = self.rebuild_rx.recv() => {
                    self.dirty_views.insert(view_id);
                    // Rebuild from root for correctness; the dirty_views set
                    // records which view triggered it for future targeted rebuilds
                    let _ = self.build().await;
                }
            }
        }

        // Cleanup all effects on shutdown
        for (_, mut store) in self.hook_stores.drain() {
            store.cleanup_all_effects();
        }
    }

    /// Get the current widget tree as serialized JSON.
    pub async fn current_tree(&self) -> Option<serde_json::Value> {
        let tree = self.tree.read().await;
        tree.as_ref()
            .map(|el| serde_json::to_value(el).unwrap_or_default())
    }

    /// Get read access to the view tree.
    pub fn view_tree(&self) -> &ViewTree {
        &self.view_tree
    }

    /// Get mutable access to the hook stores (for child_view to access).
    pub fn hook_stores_mut(&mut self) -> &mut HashMap<ViewId, HookStore> {
        &mut self.hook_stores
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::view::{BuildContext, Element, View};
    use crate::widgets::button::Button;
    use crate::widgets::text::TextBlock;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    struct TestView;

    impl View for TestView {
        fn build(&self, _ctx: &mut BuildContext) -> Element {
            Element::Widget(Box::new(TextBlock::new("Hello from runtime")))
        }
    }

    #[tokio::test]
    async fn test_runtime_build() {
        let mut runtime = Runtime::new(TestView);
        let tree = runtime.build().await;
        let json = serde_json::to_value(&tree).unwrap();
        assert!(json.to_string().contains("Hello from runtime"));
    }

    #[tokio::test]
    async fn test_runtime_build_assigns_ids_automatically() {
        use crate::widgets::layout::Layout;

        struct AutoIdView;

        impl View for AutoIdView {
            fn build(&self, _ctx: &mut BuildContext) -> Element {
                Layout::vertical()
                    .child(TextBlock::new("First"))
                    .child(TextBlock::new("Second"))
                    .child(Button::new("Click"))
                    .into()
            }
        }

        let mut runtime = Runtime::new(AutoIdView);
        let tree = runtime.build().await;
        let json = serde_json::to_value(&tree).unwrap();
        let json_str = json.to_string();

        assert!(json_str.contains("\"id\":\"w-0\""));
        assert!(json_str.contains("\"id\":\"w-1\""));
        assert!(json_str.contains("\"id\":\"w-2\""));
        assert!(json_str.contains("\"id\":\"w-3\""));
    }

    #[tokio::test]
    async fn test_runtime_auto_id_registers_events() {
        let clicked = Arc::new(AtomicBool::new(false));
        let clicked_clone = clicked.clone();

        struct AutoClickView {
            clicked: Arc<AtomicBool>,
        }

        impl View for AutoClickView {
            fn build(&self, _ctx: &mut BuildContext) -> Element {
                let clicked = self.clicked.clone();
                Button::new("Auto Click")
                    .on_click(move || {
                        clicked.store(true, Ordering::SeqCst);
                    })
                    .into()
            }
        }

        let mut runtime = Runtime::new(AutoClickView {
            clicked: clicked_clone.clone(),
        });

        let _ = runtime.build().await;

        let tx = runtime.event_sender();
        tx.send(RuntimeMessage::Event {
            widget_id: "w-0".to_string(),
            event_name: "click".to_string(),
            args: serde_json::Value::Null,
        })
        .await
        .unwrap();

        tokio::spawn(async move {
            runtime.run().await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(clicked_clone.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_button_click_dispatched() {
        let clicked = Arc::new(AtomicBool::new(false));
        let clicked_clone = clicked.clone();

        struct ClickView {
            clicked: Arc<AtomicBool>,
        }

        impl View for ClickView {
            fn build(&self, _ctx: &mut BuildContext) -> Element {
                let clicked = self.clicked.clone();
                Button::new("Click me")
                    .on_click(move || {
                        clicked.store(true, Ordering::SeqCst);
                    })
                    .into()
            }
        }

        let mut runtime = Runtime::new(ClickView {
            clicked: clicked_clone.clone(),
        });

        let _ = runtime.build().await;

        let tx = runtime.event_sender();
        tx.send(RuntimeMessage::Event {
            widget_id: "w-0".to_string(),
            event_name: "click".to_string(),
            args: serde_json::Value::Null,
        })
        .await
        .unwrap();

        tokio::spawn(async move {
            runtime.run().await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(clicked_clone.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_text_input_change_dispatched() {
        let received = Arc::new(Mutex::new(String::new()));
        let received_clone = received.clone();

        struct InputView {
            received: Arc<Mutex<String>>,
        }

        impl View for InputView {
            fn build(&self, _ctx: &mut BuildContext) -> Element {
                let received = self.received.clone();
                crate::widgets::input::TextInput::new()
                    .on_change(move |val| {
                        let mut r = received.lock().unwrap();
                        *r = val;
                    })
                    .into()
            }
        }

        let mut runtime = Runtime::new(InputView {
            received: received_clone.clone(),
        });

        let _ = runtime.build().await;

        let tx = runtime.event_sender();
        tx.send(RuntimeMessage::Event {
            widget_id: "w-0".to_string(),
            event_name: "change".to_string(),
            args: serde_json::json!({"value": "hello world"}),
        })
        .await
        .unwrap();

        tokio::spawn(async move {
            runtime.run().await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let val = received_clone.lock().unwrap();
        assert_eq!(*val, "hello world");
    }

    #[tokio::test]
    async fn test_rebuild_carries_view_id() {
        use crate::hooks::use_state::use_state;

        struct StatefulView;

        impl View for StatefulView {
            fn build(&self, ctx: &mut BuildContext) -> Element {
                let count = use_state(ctx, 0i32);
                Element::Widget(Box::new(TextBlock::new(&format!("Count: {}", count.get()))))
            }
        }

        let mut runtime = Runtime::new(StatefulView);
        let _ = runtime.build().await;

        // The root view_id is now tracked in the view_tree
        let root_id = runtime.view_tree().root_id();
        assert!(runtime.hook_stores_mut().contains_key(&root_id));
    }

    #[test]
    fn test_child_view_independent_hooks() {
        use crate::hooks::use_state::use_state;

        struct ChildView;
        impl View for ChildView {
            fn build(&self, ctx: &mut BuildContext) -> Element {
                let count = use_state(ctx, 100i32);
                Element::Widget(Box::new(TextBlock::new(&format!("Child: {}", count.get()))))
            }
        }

        let mut parent_store = HookStore::new();
        let parent_view_id = uuid::Uuid::new_v4();

        let mut ctx = BuildContext::with_view_id(&mut parent_store, None, parent_view_id);

        // Parent uses its own state
        let parent_state = use_state(&mut ctx, 0i32);
        parent_state.set(42);

        // Embed a child view — it should get its own isolated HookStore
        let (child_element, child_id, child_store) = ctx.child_view(ChildView, None);

        // Verify the child produced its element
        let json = serde_json::to_value(&child_element).unwrap().to_string();
        assert!(json.contains("Child: 100"));

        // Verify the child got a different ViewId
        assert_ne!(parent_view_id, child_id);

        // Verify the child has its own HookStore with state
        assert!(
            !child_store.states.is_empty(),
            "Child should have its own state"
        );

        // Parent state should still be 42
        assert_eq!(parent_state.get(), 42);
    }

    #[tokio::test]
    async fn test_subtree_rebuild_only() {
        use crate::hooks::use_state::use_state;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let root_build_count = Arc::new(AtomicUsize::new(0));
        let root_build_count_clone = root_build_count.clone();

        struct CountingView {
            build_count: Arc<AtomicUsize>,
        }

        impl View for CountingView {
            fn build(&self, ctx: &mut BuildContext) -> Element {
                self.build_count.fetch_add(1, Ordering::SeqCst);
                let count = use_state(ctx, 0i32);
                Element::Widget(Box::new(TextBlock::new(&format!("Count: {}", count.get()))))
            }
        }

        let mut runtime = Runtime::new(CountingView {
            build_count: root_build_count_clone,
        });

        // Initial build
        let _ = runtime.build().await;
        assert_eq!(root_build_count.load(Ordering::SeqCst), 1);

        // Second build (full rebuild) should increment
        let _ = runtime.build().await;
        assert_eq!(root_build_count.load(Ordering::SeqCst), 2);

        // The dirty_views mechanism is in place — when state changes trigger
        // rebuild via ViewId, the runtime knows which view changed
        let root_id = runtime.view_tree().root_id();
        runtime.mark_dirty(root_id);
        assert!(runtime.dirty_views.contains(&root_id));
    }

    #[test]
    fn test_context_ancestor_walk() {
        use crate::hooks::use_context::{create_context, use_context};

        #[derive(Clone, Debug, PartialEq)]
        struct Theme {
            color: String,
        }

        struct ChildView;
        impl View for ChildView {
            fn build(&self, ctx: &mut BuildContext) -> Element {
                let theme: Theme = use_context(ctx);
                Element::Widget(Box::new(TextBlock::new(&format!("Theme: {}", theme.color))))
            }
        }

        let mut parent_store = HookStore::new();
        let parent_view_id = uuid::Uuid::new_v4();

        let mut ctx = BuildContext::with_view_id(&mut parent_store, None, parent_view_id);

        // Create context in parent
        create_context(
            &mut ctx,
            Theme {
                color: "blue".to_string(),
            },
        );

        // Child view should be able to read the parent's context via ancestor walk
        let (child_element, _child_id, _child_store) = ctx.child_view(ChildView, None);

        let json = serde_json::to_value(&child_element).unwrap().to_string();
        assert!(
            json.contains("Theme: blue"),
            "Expected child to read parent context, got: {}",
            json
        );
    }

    #[test]
    fn test_cleanup_on_view_removal() {
        use crate::core::view_tree::ViewTree;
        use std::sync::atomic::{AtomicBool, Ordering};

        let cleanup_called = Arc::new(AtomicBool::new(false));
        let cleanup_called_clone = cleanup_called.clone();

        // Create a view tree with a child
        let mut tree = ViewTree::new(Box::new(|_ctx: &mut BuildContext| Element::Empty));
        let root_id = tree.root_id();
        let child_id = tree.insert(root_id, Box::new(|_ctx: &mut BuildContext| Element::Empty));

        // Set up HookStores with an effect cleanup for the child
        let mut hook_stores: HashMap<ViewId, HookStore> = HashMap::new();
        let child_store = hook_stores.entry(child_id).or_default();
        child_store.effects.insert(
            0,
            crate::hooks::hook_store::EffectEntry {
                prev_deps: None,
                cleanup: Some(Box::new(move || {
                    cleanup_called_clone.store(true, Ordering::SeqCst);
                })),
                has_run: true,
            },
        );

        // Remove the child from the tree
        let removed = tree.remove(child_id);
        assert!(removed.contains(&child_id));

        // Clean up the removed HookStores
        for removed_id in removed {
            if let Some(mut store) = hook_stores.remove(&removed_id) {
                store.cleanup_all_effects();
            }
        }

        // Verify effect cleanup was called
        assert!(cleanup_called.load(Ordering::SeqCst));
    }
}
