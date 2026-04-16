use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

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
    event_tx: mpsc::Sender<RuntimeMessage>,
    event_rx: mpsc::Receiver<RuntimeMessage>,
}

impl Runtime {
    pub fn new(root: impl View) -> Self {
        let (event_tx, event_rx) = mpsc::channel(2048);
        Runtime {
            root: Box::new(root),
            tree: Arc::new(RwLock::new(None)),
            event_tx,
            event_rx,
        }
    }

    /// Get a sender for dispatching events to the runtime.
    pub fn event_sender(&self) -> mpsc::Sender<RuntimeMessage> {
        self.event_tx.clone()
    }

    /// Build the initial view tree.
    pub async fn build(&self) -> Element {
        let mut ctx = BuildContext::new();
        let element = self.root.build(&mut ctx);
        let mut tree = self.tree.write().await;
        *tree = Some(element.clone());
        element
    }

    /// Run the event loop, processing messages until shutdown.
    pub async fn run(&mut self) {
        // Initial build
        let _ = self.build().await;

        while let Some(msg) = self.event_rx.recv().await {
            match msg {
                RuntimeMessage::Event {
                    widget_id: _,
                    event_name: _,
                    args: _,
                } => {
                    // Dispatch event to the appropriate widget handler
                    // Then rebuild the affected view subtree
                    let _ = self.build().await;
                }
                RuntimeMessage::Rebuild { view_id: _ } => {
                    let _ = self.build().await;
                }
                RuntimeMessage::Shutdown => break,
            }
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
        let runtime = Runtime::new(TestView);
        let tree = runtime.build().await;
        let json = serde_json::to_value(&tree).unwrap();
        assert!(json.to_string().contains("Hello from runtime"));
    }
}
