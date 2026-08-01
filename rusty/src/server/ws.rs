use axum::{
    body::Body,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use crate::core::runtime::RuntimeMessage;
use crate::views::view::View;

use super::download::{DownloadPayload, DownloadService};
use super::session::AppSessionStore;

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
        /// Optional navigation state. A bare `{"method":"navigate","appId":"x"}`
        /// must still deserialize instead of being silently dropped.
        #[serde(default)]
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

/// Loopback-only default: a dev server or test harness should not be reachable
/// from the local network. Callers that need external access opt in explicitly
/// via [`RustyServer::with_bind_address`].
pub const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1";

/// Minimum interval between pushes on the rebuild-signal arm.
///
/// The signal itself is not rate limited: an isolated state change still drains
/// and pushes immediately (leading edge). This only caps a *sustained* producer
/// (a `use_interval` at 1 ms, or a task calling `State::set` in a loop), which
/// would otherwise take the session write lock and re-diff the whole tree on
/// every set. One frame at 60 Hz is finer than a browser can paint.
const MIN_PUSH_INTERVAL: std::time::Duration = std::time::Duration::from_millis(16);

/// Application state shared across WebSocket connections.
pub struct AppState {
    pub session_store: AppSessionStore,
}

/// The Rusty WebSocket server for frontend communication.
pub struct RustyServer {
    port: u16,
    bind_address: String,
    root_view: Box<dyn Fn() -> Box<dyn View> + Send + Sync>,
    static_dir: Option<PathBuf>,
}

impl RustyServer {
    pub fn new<F, V>(port: u16, root_factory: F) -> Self
    where
        F: Fn() -> V + Send + Sync + 'static,
        V: View,
    {
        RustyServer {
            port,
            bind_address: DEFAULT_BIND_ADDRESS.to_string(),
            root_view: Box::new(move || Box::new(root_factory())),
            static_dir: None,
        }
    }

    /// Bind to a specific address instead of the loopback default.
    ///
    /// Pass `"0.0.0.0"` to accept connections from any interface.
    pub fn with_bind_address(mut self, address: impl Into<String>) -> Self {
        self.bind_address = address.into();
        self
    }

    /// Serve static files from the given directory at `/`.
    pub fn with_static_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.static_dir = Some(dir.into());
        self
    }

    /// Build the axum router with WebSocket support.
    pub fn router(self) -> Router {
        let root_factory: Arc<dyn Fn() -> Box<dyn View> + Send + Sync> = Arc::from(self.root_view);
        let session_store = AppSessionStore::new(root_factory);
        let state = Arc::new(AppState { session_store });

        let mut router = Router::new()
            .route("/ws", get(ws_handler))
            .route("/health", get(health_handler))
            // axum 0.8 path params use brace syntax.
            .route(
                "/rusty/download/{connection_id}/{download_id}",
                get(download_handler),
            )
            .with_state(state);

        if let Some(dir) = self.static_dir {
            router = router.fallback_service(
                tower_http::services::ServeDir::new(dir).append_index_html_on_directories(true),
            );
        }

        router
    }

    /// Start the server and listen for connections.
    /// Returns the actual bound address (useful when port is 0).
    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("{}:{}", self.bind_address, self.port);
        let router = self.router();
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        let local_addr = listener.local_addr()?;
        tracing::info!("Rusty server listening on {}", local_addr);
        println!("RUSTY_PORT={}", local_addr.port());
        axum::serve(listener, router).await?;
        Ok(())
    }

    /// Start the server and return the bound address without blocking.
    /// Useful for testing — spawns the server on a background task.
    pub async fn serve_background(self) -> Result<SocketAddr, Box<dyn std::error::Error>> {
        let addr = format!("{}:{}", self.bind_address, self.port);
        let router = self.router();
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        let local_addr = listener.local_addr()?;
        tracing::info!("Rusty server listening on {}", local_addr);
        tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        Ok(local_addr)
    }
}

/// Wrapper to make a boxed View usable.
pub struct FuncView(pub Box<dyn View + Send + Sync>);

impl View for FuncView {
    fn build(&self, ctx: &mut crate::views::view::BuildContext) -> crate::views::view::Element {
        self.0.build(ctx)
    }
}

async fn health_handler() -> &'static str {
    "ok"
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Serve a download registered by a view through `use_download`.
///
/// Downloads are keyed by connection, so a URL only resolves for the session that
/// created it. Anything unresolvable — unknown session, unparseable or unknown
/// download id, or a factory that failed — is a 404.
async fn download_handler(
    Path((connection_id, download_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let Ok(download_id) = Uuid::parse_str(&download_id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(session_arc) = state.session_store.get_session(&connection_id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    // Resolve the service and release the session lock before running the factory,
    // which may take a while and must not block the session's event loop.
    let download_service = {
        let session = session_arc.read().await;
        session.services.get::<DownloadService>()
    };
    let Some(download_service) = download_service else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let Some(response) = download_service.take(download_id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let body = match response.payload {
        DownloadPayload::Bytes(bytes) => Body::from(bytes),
        DownloadPayload::Stream(stream) => Body::from_stream(stream),
    };

    (
        [
            (header::CONTENT_TYPE, response.mime_type),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", response.file_name),
            ),
        ],
        body,
    )
        .into_response()
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Generate a unique connection ID and create an isolated session
    let connection_id = Uuid::new_v4().to_string();
    let session_arc = state
        .session_store
        .create_session(connection_id.clone())
        .await;
    let mut shutdown_rx = state.session_store.subscribe_shutdown();

    // Send initial render from this session's own runtime
    {
        let mut session = session_arc.write().await;
        session.runtime.build().await;
        if let Some(tree) = session.runtime.current_tree().await {
            let msg = ServerMessage::Refresh {
                widgets: tree.clone(),
            };
            session.reconciler.reconcile(&tree);
            if let Ok(json) = serde_json::to_string(&msg) {
                let _ = sender.send(Message::Text(json.into())).await;
            }
        }
    }
    let event_tx = session_arc.read().await.runtime.event_sender();

    // Woken when a rebuild is queued outside the request path (async hooks
    // resolving, spawned tasks calling State::set). No polling: the task parks
    // until a producer actually signals.
    let rebuild_notify = session_arc.read().await.runtime.rebuild_notifier();
    // Leading-edge debounce for the push arm: `next_push` starts in the past so
    // the first signal drains at once, and each drain arms the next window.
    let mut push_pending = false;
    let mut next_push = tokio::time::Instant::now();

    // Process incoming messages using this session's isolated runtime
    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Err(err) => {
                                tracing::warn!("Ignoring unparseable client message: {err}");
                            }
                            Ok(client_msg) => match client_msg {
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

                                    // Dispatch the queued event and rebuild before reading the
                                    // tree — nothing else drains the runtime's channels.
                                    let mut session = session_arc.write().await;
                                    session.runtime.process_pending().await;
                                    if let Some(tree) = session.runtime.current_tree().await {
                                        if let Some(patches) = session.reconciler.reconcile(&tree) {
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
                            },
                        }
                    }
                    Some(Ok(_)) => {} // Ignore non-text messages
                    _ => break, // Connection closed or error
                }
            }
            // Record that work is queued; the drain happens in the arm below so a
            // hot producer cannot make us re-diff the tree once per `State::set`.
            _ = rebuild_notify.notified(), if !push_pending => {
                push_pending = true;
            }
            _ = tokio::time::sleep_until(next_push), if push_pending => {
                push_pending = false;
                next_push = tokio::time::Instant::now() + MIN_PUSH_INTERVAL;
                let mut session = session_arc.write().await;
                if session.runtime.process_pending().await {
                    if let Some(tree) = session.runtime.current_tree().await {
                        if let Some(patches) = session.reconciler.reconcile(&tree) {
                            if !patches.is_empty() {
                                let msg = ServerMessage::Update { patches };
                                if let Ok(json) = serde_json::to_string(&msg) {
                                    let _ = sender.send(Message::Text(json.into())).await;
                                }
                            }
                        }
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                break;
            }
        }
    }

    // Clean up session on disconnect
    state.session_store.remove_session(&connection_id).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::TextBlock;

    struct Probe;

    impl crate::views::view::View for Probe {
        fn build(
            &self,
            _ctx: &mut crate::views::view::BuildContext,
        ) -> crate::views::view::Element {
            TextBlock::new("probe").into()
        }
    }

    #[tokio::test]
    async fn serve_background_binds_loopback_by_default() {
        let addr = RustyServer::new(0, || Probe)
            .serve_background()
            .await
            .expect("bind");
        assert_eq!(addr.ip(), std::net::Ipv4Addr::LOCALHOST);
    }

    #[tokio::test]
    async fn with_bind_address_overrides_the_default() {
        let addr = RustyServer::new(0, || Probe)
            .with_bind_address("0.0.0.0")
            .serve_background()
            .await
            .expect("bind");
        assert_eq!(addr.ip(), std::net::Ipv4Addr::UNSPECIFIED);
    }

    #[test]
    fn navigate_deserializes_without_state() {
        let msg: ClientMessage =
            serde_json::from_str(r#"{"method":"navigate","appId":"reports"}"#).unwrap();
        match msg {
            ClientMessage::Navigate { app_id, state } => {
                assert_eq!(app_id, "reports");
                assert_eq!(state, serde_json::Value::Null);
            }
            other => panic!("expected Navigate, got {other:?}"),
        }
    }

    #[test]
    fn navigate_still_deserializes_with_state() {
        let msg: ClientMessage =
            serde_json::from_str(r#"{"method":"navigate","appId":"reports","state":{"page":2}}"#)
                .unwrap();
        match msg {
            ClientMessage::Navigate { app_id, state } => {
                assert_eq!(app_id, "reports");
                assert_eq!(state["page"], 2);
            }
            other => panic!("expected Navigate, got {other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn push_debounce_drains_the_first_signal_immediately() {
        let notify = std::sync::Arc::new(tokio::sync::Notify::new());
        let mut push_pending = false;
        let mut next_push = tokio::time::Instant::now();
        let started = tokio::time::Instant::now();

        notify.notify_one();

        tokio::select! {
            _ = notify.notified(), if !push_pending => {
                push_pending = true;
            }
            _ = tokio::time::sleep_until(next_push), if push_pending => {
                push_pending = false;
                next_push = tokio::time::Instant::now() + MIN_PUSH_INTERVAL;
            }
        }

        assert!(push_pending, "latch arm should have won");

        tokio::select! {
            _ = notify.notified(), if !push_pending => {
                unreachable!();
            }
            _ = tokio::time::sleep_until(next_push), if push_pending => {}
        }

        assert_eq!(
            started.elapsed(),
            std::time::Duration::ZERO,
            "drain should not have waited"
        );
    }

    #[tokio::test]
    async fn push_debounce_caps_the_drain_rate_under_a_hot_producer() {
        let notify = std::sync::Arc::new(tokio::sync::Notify::new());
        let notify_clone = notify.clone();
        let mut push_pending = false;
        let mut next_push = tokio::time::Instant::now();
        let mut drains = 0;
        let mut signals = 0;

        let producer = tokio::spawn(async move {
            for _ in 0..5000 {
                notify_clone.notify_one();
                tokio::task::yield_now().await;
            }
        });

        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(200);
        loop {
            tokio::select! {
                _ = notify.notified(), if !push_pending => {
                    push_pending = true;
                    signals += 1;
                }
                _ = tokio::time::sleep_until(next_push), if push_pending => {
                    push_pending = false;
                    next_push = tokio::time::Instant::now() + MIN_PUSH_INTERVAL;
                    drains += 1;
                }
                _ = tokio::time::sleep_until(deadline) => {
                    break;
                }
            }
        }

        let _ = producer.await;
        let window = std::time::Duration::from_millis(200);
        let max_drains = (window.as_millis() / MIN_PUSH_INTERVAL.as_millis()) + 2;
        assert!(
            drains <= max_drains as usize,
            "drains ({drains}) should be capped by the window"
        );
        assert!(
            signals > drains * 4,
            "not enough signals ({signals}) to prove coalescing � producer may be broken"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn push_debounce_never_loses_a_signal_across_the_window() {
        let notify = std::sync::Arc::new(tokio::sync::Notify::new());
        let mut push_pending = false;
        let next_push = tokio::time::Instant::now();

        notify.notify_one();

        tokio::select! {
            _ = notify.notified(), if !push_pending => {
                push_pending = true;
            }
            _ = tokio::time::sleep_until(next_push), if push_pending => {}
        }

        notify.notify_one();

        tokio::select! {
            _ = notify.notified(), if !push_pending => {
                unreachable!("latch arm should be disabled");
            }
            _ = tokio::time::sleep_until(next_push), if push_pending => {}
        }

        let resolved =
            tokio::time::timeout(std::time::Duration::from_millis(10), notify.notified()).await;
        assert!(
            resolved.is_ok(),
            "notified() should still resolve � the stored permit survived"
        );
    }

    #[test]
    fn event_still_requires_all_fields() {
        assert!(serde_json::from_str::<ClientMessage>(
            r#"{"method":"event","widgetId":"btn-1","eventName":"click","args":[]}"#
        )
        .is_ok());
        assert!(serde_json::from_str::<ClientMessage>(
            r#"{"method":"event","widgetId":"btn-1","eventName":"click"}"#
        )
        .is_err());
    }
}
