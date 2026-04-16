use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

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
    root: Box<dyn View>,
    tree: Arc<RwLock<Option<Element>>>,
    hook_stores: HashMap<ViewId, HookStore>,
    root_view_id: ViewId,
    event_tx: mpsc::Sender<RuntimeMessage>,
    event_rx: mpsc::Receiver<RuntimeMessage>,
    rebuild_tx: mpsc::Sender<()>,
    rebuild_rx: mpsc::Receiver<()>,
}

impl Runtime {
    pub fn new(root: impl View) -> Self {
        let (event_tx, event_rx) = mpsc::channel(2048);
        let (rebuild_tx, rebuild_rx) = mpsc::channel(256);
        Runtime {
            root: Box::new(root),
            tree: Arc::new(RwLock::new(None)),
            hook_stores: HashMap::new(),
            root_view_id: uuid::Uuid::new_v4(),
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
    pub fn rebuild_sender(&self) -> mpsc::Sender<()> {
        self.rebuild_tx.clone()
    }

    /// Build the view tree, persisting hook state across re-renders.
    pub async fn build(&mut self) -> Element {
        let store = self.hook_stores.entry(self.root_view_id).or_default();
        let mut ctx = BuildContext::new(store, Some(self.rebuild_tx.clone()));
        ctx.reset();
        let element = self.root.build(&mut ctx);

        // Drain and execute effects
        let effects = ctx.drain_effects();
        let store = self.hook_stores.get_mut(&self.root_view_id).unwrap();
        for effect_record in effects {
            let idx = effect_record.hook_index;
            // Run cleanup from previous effect execution
            if let Some(entry) = store.effects.get_mut(&idx) {
                if let Some(cleanup) = entry.cleanup.take() {
                    cleanup();
                }
            }
            // Execute the effect and store any cleanup
            let cleanup = (effect_record.callback)();
            if let Some(entry) = store.effects.get_mut(&idx) {
                entry.cleanup = cleanup;
            }
        }

        let mut tree = self.tree.write().await;
        *tree = Some(element.clone());
        element
    }

    /// Run the event loop, processing messages until shutdown.
    pub async fn run(&mut self) {
        // Initial build
        let _ = self.build().await;

        loop {
            tokio::select! {
                msg = self.event_rx.recv() => {
                    match msg {
                        Some(RuntimeMessage::Event { .. }) => {
                            let _ = self.build().await;
                        }
                        Some(RuntimeMessage::Rebuild { .. }) => {
                            let _ = self.build().await;
                        }
                        Some(RuntimeMessage::Shutdown) | None => break,
                    }
                }
                _ = self.rebuild_rx.recv() => {
                    // State change triggered a rebuild
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::view::{BuildContext, Element, View};
    use crate::widgets::text::TextBlock;

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
}
