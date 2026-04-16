use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::core::event_registry::EventRegistry;
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
    event_registry: Arc<RwLock<EventRegistry>>,
    event_tx: mpsc::Sender<RuntimeMessage>,
    event_rx: mpsc::Receiver<RuntimeMessage>,
}

impl Runtime {
    pub fn new(root: impl View) -> Self {
        let (event_tx, event_rx) = mpsc::channel(2048);
        Runtime {
            root: Box::new(root),
            tree: Arc::new(RwLock::new(None)),
            event_registry: Arc::new(RwLock::new(EventRegistry::new())),
            event_tx,
            event_rx,
        }
    }

    /// Get a sender for dispatching events to the runtime.
    pub fn event_sender(&self) -> mpsc::Sender<RuntimeMessage> {
        self.event_tx.clone()
    }

    /// Build the view tree and extract the event registry.
    pub async fn build(&self) -> Element {
        let mut ctx = BuildContext::new();
        let element = self.root.build(&mut ctx);

        // Extract the event registry populated during build
        let registry = ctx.take_event_registry();
        let mut reg = self.event_registry.write().await;
        *reg = registry;

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
                    widget_id,
                    event_name,
                    args,
                } => {
                    // Dispatch event to the registered handler
                    {
                        let registry = self.event_registry.read().await;
                        registry.dispatch(&widget_id, &event_name, args);
                    }
                    // Rebuild after handler may have mutated state
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
        let runtime = Runtime::new(TestView);
        let tree = runtime.build().await;
        let json = serde_json::to_value(&tree).unwrap();
        assert!(json.to_string().contains("Hello from runtime"));
    }

    #[tokio::test]
    async fn test_button_click_dispatched() {
        let clicked = Arc::new(AtomicBool::new(false));
        let clicked_clone = clicked.clone();

        struct ClickView {
            clicked: Arc<AtomicBool>,
        }

        impl View for ClickView {
            fn build(&self, ctx: &mut BuildContext) -> Element {
                let clicked = self.clicked.clone();
                let btn = Button::new("Click me")
                    .on_click(move || {
                        clicked.store(true, Ordering::SeqCst);
                    })
                    .build(ctx);
                Element::Widget(Box::new(btn))
            }
        }

        let mut runtime = Runtime::new(ClickView {
            clicked: clicked_clone.clone(),
        });

        // Initial build to populate registry
        let _ = runtime.build().await;

        // Send click event
        let tx = runtime.event_sender();
        tx.send(RuntimeMessage::Event {
            widget_id: "w-0".to_string(),
            event_name: "click".to_string(),
            args: serde_json::Value::Null,
        })
        .await
        .unwrap();

        // Process the event
        tokio::spawn(async move {
            runtime.run().await;
        });

        // Give time for the event to be processed
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
            fn build(&self, ctx: &mut BuildContext) -> Element {
                let received = self.received.clone();
                let input = crate::widgets::input::TextInput::new()
                    .on_change(move |val| {
                        let mut r = received.lock().unwrap();
                        *r = val;
                    })
                    .build(ctx);
                Element::Widget(Box::new(input))
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
}
