use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::reconciler::Reconciler;
use crate::core::runtime::{Runtime, RuntimeMessage};
use crate::views::view::View;

/// Messages sent from client to server.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum ClientMessage {
    #[serde(rename = "event")]
    Event {
        #[serde(rename = "widgetId")]
        widget_id: String,
        #[serde(rename = "eventName")]
        event_name: String,
        args: serde_json::Value,
    },
    #[serde(rename = "navigate")]
    Navigate {
        #[serde(rename = "appId")]
        app_id: String,
        state: serde_json::Value,
    },
}

/// Messages sent from server to client.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum ServerMessage {
    #[serde(rename = "refresh")]
    Refresh { widgets: serde_json::Value },
    #[serde(rename = "update")]
    Update {
        patches: Vec<crate::core::diff::Patch>,
    },
}

/// Application state shared across WebSocket connections.
pub struct AppState {
    pub runtime: Arc<RwLock<Runtime>>,
}

/// The Rusty WebSocket server for frontend communication.
pub struct RustyServer {
    port: u16,
    root_view: Box<dyn Fn() -> Box<dyn View> + Send + Sync>,
}

impl RustyServer {
    pub fn new<F, V>(port: u16, root_factory: F) -> Self
    where
        F: Fn() -> V + Send + Sync + 'static,
        V: View,
    {
        RustyServer {
            port,
            root_view: Box::new(move || Box::new(root_factory())),
        }
    }

    /// Build the axum router with WebSocket support.
    pub fn router(self) -> Router {
        let runtime = Runtime::new(FuncView((self.root_view)()));
        let state = Arc::new(AppState {
            runtime: Arc::new(RwLock::new(runtime)),
        });

        Router::new()
            .route("/ws", get(ws_handler))
            .route("/health", get(health_handler))
            .with_state(state)
    }

    /// Start the server and listen for connections.
    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("0.0.0.0:{}", self.port);
        let router = self.router();
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        tracing::info!("Rusty server listening on {}", addr);
        axum::serve(listener, router).await?;
        Ok(())
    }
}

/// Wrapper to make a boxed View usable.
struct FuncView(Box<dyn View>);

impl View for FuncView {
    fn build(&self, ctx: &mut crate::views::view::BuildContext) -> crate::views::view::Element {
        self.0.build(ctx)
    }
}

// Make FuncView Send + Sync safe (View trait already requires it)
unsafe impl Send for FuncView {}
unsafe impl Sync for FuncView {}

async fn health_handler() -> &'static str {
    "ok"
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut reconciler = Reconciler::new();

    // Send initial render
    let runtime = state.runtime.read().await;
    if let Some(tree) = runtime.current_tree().await {
        let msg = ServerMessage::Refresh {
            widgets: tree.clone(),
        };
        reconciler.reconcile(&tree);
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = sender.send(Message::Text(json.into())).await;
        }
    }
    let event_tx = runtime.event_sender();
    drop(runtime);

    // Process incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                match client_msg {
                    ClientMessage::Event {
                        widget_id,
                        event_name,
                        args,
                    } => {
                        let _ = event_tx
                            .send(RuntimeMessage::Event {
                                widget_id,
                                event_name,
                                args,
                            })
                            .await;

                        // After event, get updated tree and send diff
                        let runtime = state.runtime.read().await;
                        if let Some(tree) = runtime.current_tree().await {
                            if let Some(patches) = reconciler.reconcile(&tree) {
                                if !patches.is_empty() {
                                    let msg = ServerMessage::Update { patches };
                                    if let Ok(json) = serde_json::to_string(&msg) {
                                        let _ = sender.send(Message::Text(json.into())).await;
                                    }
                                }
                            }
                        }
                    }
                    ClientMessage::Navigate { .. } => {
                        // Navigation handling (future)
                    }
                }
            }
        }
    }
}
