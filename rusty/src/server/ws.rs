use axum::{
    body::Body,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use bytes::BytesMut;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use crate::core::apps::{AppFactory, AppIds, AppRegistry};
use crate::core::runtime::RuntimeMessage;
use crate::core::services::ServiceRegistry;
use crate::views::view::View;

use super::download::{DownloadPayload, DownloadService};
use super::session::{AppSession, AppSessionStore};
use super::upload::{
    accepts, UploadError, UploadEvent, UploadService, UploadSlot, UploadedFile,
    DEFAULT_MAX_UPLOAD_BYTES, MULTIPART_ENVELOPE_ALLOWANCE,
};

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

/// Query parameters accepted when opening the WebSocket at `/ws`.
#[derive(Debug, Default, Deserialize)]
pub struct ConnectParams {
    /// The app to mount for this connection. Unknown or absent falls back to the
    /// default app.
    #[serde(rename = "appId")]
    pub app_id: Option<String>,
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
    apps: AppRegistry,
    services: ServiceRegistry,
    static_dir: Option<PathBuf>,
    max_upload_bytes: u64,
}

impl RustyServer {
    /// Single-app server: `root_factory` is registered under [`AppIds::DEFAULT`], so
    /// every connection mounts it.
    pub fn new<F, V>(port: u16, root_factory: F) -> Self
    where
        F: Fn() -> V + Send + Sync + 'static,
        V: View,
    {
        let factory: AppFactory = Arc::new(move || Box::new(root_factory()));
        let mut apps = AppRegistry::new();
        apps.register(AppIds::DEFAULT, "App", factory);

        RustyServer {
            port,
            bind_address: DEFAULT_BIND_ADDRESS.to_string(),
            apps,
            services: ServiceRegistry::new(),
            static_dir: None,
            max_upload_bytes: DEFAULT_MAX_UPLOAD_BYTES,
        }
    }

    /// Server with no apps yet — add them with [`RustyServer::with_app`].
    ///
    /// A connection arriving before any app is registered gets an empty
    /// [`AppIds::ERROR_NOT_FOUND`] session rather than a closed socket.
    pub fn empty(port: u16) -> Self {
        RustyServer {
            port,
            bind_address: DEFAULT_BIND_ADDRESS.to_string(),
            apps: AppRegistry::new(),
            services: ServiceRegistry::new(),
            static_dir: None,
            max_upload_bytes: DEFAULT_MAX_UPLOAD_BYTES,
        }
    }

    /// Register an app under `id`, selectable with `/ws?appId=<id>` and by a
    /// `navigate` message. The first app registered becomes the default unless one is
    /// registered under [`AppIds::DEFAULT`].
    pub fn with_app<F, V>(
        mut self,
        id: impl Into<String>,
        title: impl Into<String>,
        factory: F,
    ) -> Self
    where
        F: Fn() -> V + Send + Sync + 'static,
        V: View,
    {
        let factory: AppFactory = Arc::new(move || Box::new(factory()));
        self.apps.register(id, title, factory);
        self
    }

    /// Register a server-level service, resolvable with `use_service` from every app on
    /// every connection.
    ///
    /// Framework services (`AppContext`, `DownloadService`, the session `SignalRegistry`)
    /// are registered per connection and always win over a value registered here.
    pub fn with_service<T: Send + Sync + 'static>(self, value: T) -> Self {
        self.services.register(Arc::new(value));
        self
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

    /// Cap the request body the upload endpoint will accept. Defaults to
    /// [`DEFAULT_MAX_UPLOAD_BYTES`] (32 MiB).
    ///
    /// This is the outer transport limit, applied as a `DefaultBodyLimit` layer on
    /// the upload route alone — the WebSocket and download routes keep axum's own
    /// 2 MiB default. A body over the limit is refused by the layer with a bare
    /// `413` and no [`UploadError`], so per-file limits belong in
    /// [`UploadConstraints::max_bytes`](crate::server::upload::UploadConstraints::max_bytes),
    /// which reports a reason the view can render. Keep this at or above the largest
    /// `max_bytes` any view asks for.
    pub fn with_max_upload_bytes(mut self, max_upload_bytes: u64) -> Self {
        self.max_upload_bytes = max_upload_bytes;
        self
    }

    /// Build the axum router with WebSocket support.
    pub fn router(self) -> Router {
        let max_upload_bytes = self.max_upload_bytes;
        let session_store =
            AppSessionStore::with_apps(Arc::new(self.apps), Arc::new(self.services));
        let state = Arc::new(AppState { session_store });

        let mut router = Router::new()
            .route("/ws", get(ws_handler))
            .route("/health", get(health_handler))
            // axum 0.8 path params use brace syntax.
            .route(
                "/rusty/download/{connection_id}/{download_id}",
                get(download_handler),
            )
            .route(
                "/rusty/upload/{connection_id}/{upload_id}",
                // The raised body limit applies to this route only.
                post(upload_handler).layer(DefaultBodyLimit::max(
                    usize::try_from(max_upload_bytes).unwrap_or(usize::MAX),
                )),
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

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<ConnectParams>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, params.app_id))
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

/// Receive a file for an upload slot registered by a view through `use_upload`.
///
/// The body shape is fixed by the client half that already exists:
/// `uploadFileWithProgress` POSTs a `multipart/form-data` body with a single field
/// named `file`. Slots are keyed by connection, so a URL only resolves for the
/// session that created it, and anything unresolvable — unknown session, unparseable
/// or unknown upload id, no `UploadService` — is a 404 with no observer to notify.
///
/// Once a slot *is* resolved, every failure reports itself to the view through
/// `UploadEvent::Failed` before answering, because the browser only sees the status
/// code and cannot render a reason.
async fn upload_handler(
    Path((connection_id, upload_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    // `Multipart` consumes the body, so it must come last.
    mut multipart: Multipart,
) -> Response {
    let Ok(upload_id) = Uuid::parse_str(&upload_id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(session_arc) = state.session_store.get_session(&connection_id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    // Resolve the service and release the session lock before reading the body: an
    // upload can take a while and must not block the session's event loop.
    let upload_service = {
        let session = session_arc.read().await;
        session.services.get::<UploadService>()
    };
    let Some(upload_service) = upload_service else {
        return StatusCode::NOT_FOUND.into_response();
    };
    // Unlike a download, resolving does not consume the slot — a view can accept a
    // second file through the same URL.
    let Some(slot) = upload_service.slot(upload_id) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let total = headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());

    // Reject a body that cannot possibly fit before reading any of it. See
    // MULTIPART_ENVELOPE_ALLOWANCE for why this is not a bare `total > max`.
    if let (Some(max_bytes), Some(total)) = (slot.constraints.max_bytes, total) {
        if total > max_bytes.saturating_add(MULTIPART_ENVELOPE_ALLOWANCE) {
            return reject(
                &slot,
                UploadError::TooLarge {
                    limit: max_bytes,
                    actual: total,
                },
            );
        }
    }

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            // Every field consumed and none of them was the file.
            Ok(None) => return reject(&slot, UploadError::NoFile),
            Err(error) => return reject(&slot, UploadError::Transport(error.to_string())),
        };

        if field.name() != Some("file") {
            continue;
        }

        // Copy the metadata out before reading chunks, which borrows the field
        // mutably.
        let file_name = field.file_name().unwrap_or("upload").to_string();
        let mime_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        if !accepts(&slot.constraints.accept, &mime_type, &file_name) {
            return reject(
                &slot,
                UploadError::RejectedMimeType {
                    mime_type,
                    accept: slot.constraints.accept.clone(),
                },
            );
        }

        return read_file_field(field, slot, file_name, mime_type, total).await;
    }
}

/// Drain one multipart field into memory, reporting progress and enforcing the
/// size constraints as the bytes arrive.
///
/// Chunk by chunk rather than `Field::bytes()`: that is what makes progress
/// reporting and mid-flight cancellation possible, and it lets an oversize body be
/// rejected without buffering all of it.
async fn read_file_field(
    mut field: axum::extract::multipart::Field<'_>,
    slot: UploadSlot,
    file_name: String,
    mime_type: String,
    total: Option<u64>,
) -> Response {
    let mut content = BytesMut::new();
    let mut received = 0u64;

    loop {
        if slot.is_cancelled() {
            return reject(&slot, UploadError::Cancelled);
        }

        match field.chunk().await {
            Ok(Some(chunk)) => {
                received += chunk.len() as u64;
                if let Some(max_bytes) = slot.constraints.max_bytes {
                    if received > max_bytes {
                        return reject(
                            &slot,
                            UploadError::TooLarge {
                                limit: max_bytes,
                                actual: received,
                            },
                        );
                    }
                }
                content.extend_from_slice(&chunk);
                slot.emit(UploadEvent::Progress { received, total });
            }
            Ok(None) => break,
            Err(error) => return reject(&slot, UploadError::Transport(error.to_string())),
        }
    }

    if let Some(min_bytes) = slot.constraints.min_bytes {
        if received < min_bytes {
            return reject(
                &slot,
                UploadError::TooSmall {
                    limit: min_bytes,
                    actual: received,
                },
            );
        }
    }

    let body = serde_json::json!({
        "fileName": file_name,
        "mimeType": mime_type,
        "size": content.len(),
    });
    slot.emit(UploadEvent::Completed(UploadedFile {
        file_name,
        mime_type,
        content: content.freeze(),
    }));

    (StatusCode::OK, axum::Json(body)).into_response()
}

/// Tell the view why the upload failed, then answer with the matching status.
///
/// The client reads only the status code, so the observer call is the only way the
/// reason reaches the browser at all — as rendered view state.
fn reject(slot: &UploadSlot, error: UploadError) -> Response {
    let status = error.status_code();
    tracing::debug!(%error, "upload rejected");
    slot.emit(UploadEvent::Failed(error));
    status.into_response()
}

/// Build the session's tree, send it as a full `Refresh`, and reset the reconciler
/// baseline to what was just sent.
///
/// Used for the initial render and after a navigation: in both cases the client holds
/// no tree that `Update` patches could apply to.
async fn send_refresh(
    session_arc: &Arc<tokio::sync::RwLock<AppSession>>,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
) {
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

async fn handle_socket(socket: WebSocket, state: Arc<AppState>, app_id: Option<String>) {
    let (mut sender, mut receiver) = socket.split();

    // Generate a unique connection ID and create an isolated session
    let connection_id = Uuid::new_v4().to_string();
    let session_arc = state
        .session_store
        .create_session_for_app(connection_id.clone(), app_id.as_deref())
        .await;
    let mut shutdown_rx = state.session_store.subscribe_shutdown();

    // Send initial render from this session's own runtime
    send_refresh(&session_arc, &mut sender).await;
    let mut event_tx = session_arc.read().await.runtime.event_sender();

    // Woken when a rebuild is queued outside the request path (async hooks
    // resolving, spawned tasks calling State::set). No polling: the task parks
    // until a producer actually signals.
    // `mut` because a Navigate swaps the runtime out from under us; see below.
    let mut rebuild_notify = session_arc.read().await.runtime.rebuild_notifier();
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
                                ClientMessage::Navigate { app_id, .. } => {
                                    // Swap the mounted app for this session. An unknown id
                                    // must not kill the connection - warn and hold position.
                                    if state
                                        .session_store
                                        .navigate_session(&session_arc, &app_id)
                                        .await
                                    {
                                        // The old runtime (and with it the event sender and
                                        // the rebuild notifier) is gone.
                                        {
                                            let session = session_arc.read().await;
                                            event_tx = session.runtime.event_sender();
                                            rebuild_notify = session.runtime.rebuild_notifier();
                                        }
                                        // The whole tree was replaced, so the old reconciler
                                        // baseline no longer applies: send a full Refresh
                                        // rather than Update patches.
                                        send_refresh(&session_arc, &mut sender).await;
                                    } else {
                                        tracing::warn!(
                                            "Navigate to unknown app id '{}' ignored; staying on '{}'",
                                            app_id,
                                            session_arc.read().await.app_id
                                        );
                                    }
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
        let signal_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let signal_count_clone = signal_count.clone();
        let mut push_pending = false;
        let mut next_push = tokio::time::Instant::now();
        let mut drains = 0;

        let producer = tokio::spawn(async move {
            for _ in 0..5000 {
                notify_clone.notify_one();
                signal_count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                tokio::task::yield_now().await;
            }
        });

        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(200);
        loop {
            tokio::select! {
                _ = notify.notified(), if !push_pending => {
                    push_pending = true;
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
        let signals = signal_count.load(std::sync::atomic::Ordering::Relaxed);
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

    // --- App routing over a live socket ---

    use tokio::net::TcpStream;
    use tokio_tungstenite::tungstenite::Message as WsMessage;
    use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

    type Client = WebSocketStream<MaybeTlsStream<TcpStream>>;

    struct Label(&'static str);

    impl crate::views::view::View for Label {
        fn build(
            &self,
            _ctx: &mut crate::views::view::BuildContext,
        ) -> crate::views::view::Element {
            TextBlock::new(self.0).into()
        }
    }

    /// `RustyServer::serve_background` reads its own `bind_address` field, which these
    /// tests bypass by driving the router directly. Bind 127.0.0.1 explicitly: binding
    /// all interfaces is blocked in some sandboxes.
    async fn serve_on_loopback(router: Router) -> SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        addr
    }

    /// `alpha` (default, registered first) and `beta`.
    fn two_app_router() -> Router {
        RustyServer::empty(0)
            .with_app("alpha", "Alpha", || Label("alpha-view"))
            .with_app("beta", "Beta", || Label("beta-view"))
            .router()
    }

    async fn connect(addr: SocketAddr, query: &str) -> Client {
        let url = format!("ws://{addr}/ws{query}");
        let (client, _) = connect_async(&url).await.expect("websocket handshake");
        client
    }

    /// Read the next `refresh` and return its widget tree as a string.
    ///
    /// Times out rather than hanging the suite: a missing refresh is a test failure,
    /// not a reason to wait forever.
    async fn next_refresh(client: &mut Client) -> String {
        let text = next_message(client).await;
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["method"], "refresh", "expected a refresh, got {text}");
        value["widgets"].to_string()
    }

    async fn next_message(client: &mut Client) -> String {
        let deadline = std::time::Duration::from_secs(5);
        loop {
            let msg = tokio::time::timeout(deadline, client.next())
                .await
                .expect("timed out waiting for a server message")
                .expect("stream ended")
                .expect("websocket error");
            if let WsMessage::Text(text) = msg {
                return text.to_string();
            }
        }
    }

    /// Assert the server sends nothing for `millis` - used where a message would be a bug.
    async fn assert_quiet(client: &mut Client, millis: u64) {
        let quiet =
            tokio::time::timeout(std::time::Duration::from_millis(millis), client.next()).await;
        if let Ok(Some(Ok(WsMessage::Text(text)))) = quiet {
            panic!("expected silence, got {text}");
        }
    }

    async fn send_navigate(client: &mut Client, app_id: &str) {
        let json = format!(r#"{{"method":"navigate","appId":"{app_id}","state":null}}"#);
        client.send(WsMessage::Text(json.into())).await.unwrap();
    }

    #[tokio::test]
    async fn test_initial_connection_serves_default_app() {
        let addr = serve_on_loopback(two_app_router()).await;
        let mut client = connect(addr, "").await;

        let tree = next_refresh(&mut client).await;
        assert!(tree.contains("alpha-view"), "got {tree}");
    }

    #[tokio::test]
    async fn test_app_id_query_param_selects_the_app() {
        let addr = serve_on_loopback(two_app_router()).await;
        let mut client = connect(addr, "?appId=beta").await;

        let tree = next_refresh(&mut client).await;
        assert!(tree.contains("beta-view"), "got {tree}");
        assert!(!tree.contains("alpha-view"), "got {tree}");
    }

    #[tokio::test]
    async fn test_unknown_app_id_query_param_falls_back_to_default() {
        let addr = serve_on_loopback(two_app_router()).await;
        let mut client = connect(addr, "?appId=does-not-exist").await;

        // A bookmarked dead link should still land somewhere.
        let tree = next_refresh(&mut client).await;
        assert!(tree.contains("alpha-view"), "got {tree}");
    }

    #[tokio::test]
    async fn test_navigate_switches_app_and_sends_a_refresh() {
        let addr = serve_on_loopback(two_app_router()).await;
        let mut client = connect(addr, "").await;
        assert!(next_refresh(&mut client).await.contains("alpha-view"));

        send_navigate(&mut client, "beta").await;

        // A full refresh, not update patches: the client's tree was replaced wholesale.
        let tree = next_refresh(&mut client).await;
        assert!(tree.contains("beta-view"), "got {tree}");
    }

    #[tokio::test]
    async fn test_navigate_back_and_forth() {
        let addr = serve_on_loopback(two_app_router()).await;
        let mut client = connect(addr, "").await;
        assert!(next_refresh(&mut client).await.contains("alpha-view"));

        send_navigate(&mut client, "beta").await;
        assert!(next_refresh(&mut client).await.contains("beta-view"));

        send_navigate(&mut client, "alpha").await;
        assert!(next_refresh(&mut client).await.contains("alpha-view"));
    }

    #[tokio::test]
    async fn test_navigate_to_unknown_app_keeps_connection_and_tree() {
        let addr = serve_on_loopback(two_app_router()).await;
        let mut client = connect(addr, "").await;
        assert!(next_refresh(&mut client).await.contains("alpha-view"));

        send_navigate(&mut client, "does-not-exist").await;
        assert_quiet(&mut client, 300).await;

        // The socket is still usable: a valid navigate still works.
        send_navigate(&mut client, "beta").await;
        assert!(next_refresh(&mut client).await.contains("beta-view"));
    }

    #[tokio::test]
    async fn test_navigation_does_not_affect_other_connections() {
        let addr = serve_on_loopback(two_app_router()).await;
        let mut a = connect(addr, "").await;
        let mut b = connect(addr, "").await;
        assert!(next_refresh(&mut a).await.contains("alpha-view"));
        assert!(next_refresh(&mut b).await.contains("alpha-view"));

        send_navigate(&mut a, "beta").await;
        assert!(next_refresh(&mut a).await.contains("beta-view"));

        // B stays where it was and hears nothing about A's navigation.
        assert_quiet(&mut b, 300).await;
    }

    #[tokio::test]
    async fn test_legacy_new_constructor_still_serves_its_root_view() {
        let addr = serve_on_loopback(RustyServer::new(0, || Label("legacy-view")).router()).await;
        let mut client = connect(addr, "").await;

        let tree = next_refresh(&mut client).await;
        assert!(tree.contains("legacy-view"), "got {tree}");
    }

    #[tokio::test]
    async fn test_services_resolve_over_a_live_connection() {
        struct Greeting(&'static str);

        struct GreetingView;

        impl crate::views::view::View for GreetingView {
            fn build(
                &self,
                ctx: &mut crate::views::view::BuildContext,
            ) -> crate::views::view::Element {
                let greeting = crate::hooks::use_service::<Greeting>(ctx);
                TextBlock::new(greeting.0).into()
            }
        }

        let router = RustyServer::empty(0)
            .with_app("greeter", "Greeter", || GreetingView)
            .with_service(Greeting("service-value"))
            .router();
        let addr = serve_on_loopback(router).await;
        let mut client = connect(addr, "").await;

        let tree = next_refresh(&mut client).await;
        assert!(tree.contains("service-value"), "got {tree}");
    }

    #[tokio::test]
    async fn test_navigate_with_state_omitted_still_navigates() {
        let addr = serve_on_loopback(two_app_router()).await;
        let mut client = connect(addr, "").await;
        assert!(next_refresh(&mut client).await.contains("alpha-view"));

        // No `state` key at all - without #[serde(default)] this is dropped silently.
        client
            .send(WsMessage::Text(
                r#"{"method":"navigate","appId":"beta"}"#.into(),
            ))
            .await
            .unwrap();

        assert!(next_refresh(&mut client).await.contains("beta-view"));
    }

    #[tokio::test]
    async fn test_events_dispatch_to_the_app_mounted_after_navigation() {
        struct CounterView;

        impl crate::views::view::View for CounterView {
            fn build(
                &self,
                ctx: &mut crate::views::view::BuildContext,
            ) -> crate::views::view::Element {
                let count = crate::hooks::use_state(ctx, 0i32);
                let setter = count.clone();
                // A Layout root, not Element::Fragment: a tagged newtype variant wrapping
                // a sequence cannot be serialized, so a Fragment root arrives as null.
                crate::widgets::Layout::vertical()
                    .children(vec![
                        crate::widgets::TextBlock::new(&format!("b:{}", count.get())).into(),
                        crate::widgets::Button::new("inc")
                            .on_click(move || setter.set(setter.get() + 1))
                            .into(),
                    ])
                    .into()
            }
        }

        let router = RustyServer::empty(0)
            .with_app("alpha", "Alpha", || Label("alpha-view"))
            .with_app("beta", "Beta", || CounterView)
            .router();
        let addr = serve_on_loopback(router).await;
        let mut client = connect(addr, "").await;
        assert!(next_refresh(&mut client).await.contains("alpha-view"));

        send_navigate(&mut client, "beta").await;
        let after_nav = next_refresh(&mut client).await;
        assert!(after_nav.contains("b:0"), "got {after_nav}");

        // Read the button's id out of the tree the server just sent: ids restart per
        // runtime, and the root Layout takes w-0.
        let tree: serde_json::Value = serde_json::from_str(&after_nav).unwrap();
        let button_id = tree["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|child| child["type"] == "button")
            .and_then(|child| child["id"].as_str())
            .expect("the beta app should render a button")
            .to_string();

        let event = format!(
            r#"{{"method":"event","widgetId":"{button_id}","eventName":"click","args":[]}}"#
        );
        client.send(WsMessage::Text(event.into())).await.unwrap();

        // The event reached the post-navigation runtime, which means handle_socket
        // re-read the event sender after the swap.
        let update = next_message(&mut client).await;
        assert!(update.contains("b:1"), "got {update}");
    }

    #[tokio::test]
    async fn test_health_endpoint_still_responds() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let addr = serve_on_loopback(two_app_router()).await;
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("200 OK"), "got {response}");
        assert!(response.trim_end().ends_with("ok"), "got {response}");
    }

    // --- Multipart upload through the real endpoint ---

    use crate::server::upload::UploadConstraints;

    /// Renders its own upload URL, and once a file lands, the name and bytes that
    /// reached view state — the only way a test on the far side of the socket can
    /// see what the endpoint handed the hook.
    struct UploadView {
        constraints: UploadConstraints,
    }

    impl crate::views::view::View for UploadView {
        fn build(&self, ctx: &mut crate::views::view::BuildContext) -> crate::views::view::Element {
            let upload = crate::hooks::use_upload(ctx, self.constraints.clone());
            let outcome = match upload.file.get() {
                Some(file) => format!(
                    "got:{}:{}:{}",
                    file.file_name,
                    file.mime_type,
                    String::from_utf8_lossy(&file.content)
                ),
                None => format!("status:{:?}", upload.status.get()),
            };

            crate::widgets::Layout::vertical()
                .children(vec![
                    TextBlock::new(&upload.url.get().unwrap_or_default()).into(),
                    TextBlock::new(&outcome).into(),
                ])
                .into()
        }
    }

    fn upload_router(constraints: UploadConstraints) -> Router {
        RustyServer::empty(0)
            .with_app("uploader", "Uploader", move || UploadView {
                constraints: constraints.clone(),
            })
            .router()
    }

    /// Read messages until one carries the slot URL the mount effect published.
    ///
    /// It arrives in an `update`, not the initial `refresh`: the effect runs after
    /// the first build, so the first tree still has `url == None`.
    async fn next_upload_url(client: &mut Client) -> String {
        for _ in 0..5 {
            let text = next_message(client).await;
            if let Some(start) = text.find("/rusty/upload/") {
                let rest = &text[start..];
                let end = rest
                    .find(|c: char| !c.is_ascii_alphanumeric() && !"/-_".contains(c))
                    .unwrap_or(rest.len());
                return rest[..end].to_string();
            }
        }
        panic!("the upload url never reached the client");
    }

    /// Read messages until one contains `needle`, which is how a test observes the
    /// view state the endpoint's observer wrote.
    async fn wait_for_text(client: &mut Client, needle: &str) -> String {
        for _ in 0..10 {
            let text = next_message(client).await;
            if text.contains(needle) {
                return text;
            }
        }
        panic!("the client never received {needle}");
    }

    /// POST a single-field `multipart/form-data` body and return the raw response.
    ///
    /// Hand-rolled over `TcpStream` rather than adding an HTTP client
    /// dev-dependency, the same way `test_health_endpoint_still_responds` does it.
    /// The body shape mirrors what the browser's `FormData` produces.
    async fn post_multipart(
        addr: SocketAddr,
        path: &str,
        field_name: &str,
        file_name: &str,
        content_type: &str,
        content: &[u8],
    ) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        const BOUNDARY: &str = "rustyTestBoundary";

        let mut body = Vec::new();
        body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{field_name}\"; filename=\"{file_name}\"\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
        body.extend_from_slice(content);
        body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());

        let mut request = format!(
            "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: multipart/form-data; \
             boundary={BOUNDARY}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        request.extend_from_slice(&body);

        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream.write_all(&request).await.unwrap();

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        String::from_utf8_lossy(&response).to_string()
    }

    #[tokio::test]
    async fn test_upload_accepts_a_file_and_the_bytes_reach_view_state() {
        let addr = serve_on_loopback(upload_router(UploadConstraints::new())).await;
        let mut client = connect(addr, "").await;
        let url = next_upload_url(&mut client).await;

        let response = post_multipart(
            addr,
            &url,
            "file",
            "notes.csv",
            "text/csv",
            b"id,name\n1,alice",
        )
        .await;

        assert!(response.contains("200 OK"), "got {response}");
        assert!(
            response.contains("\"fileName\":\"notes.csv\""),
            "got {response}"
        );
        assert!(response.contains("\"size\":15"), "got {response}");

        // The bytes made it all the way into the view's own state, not just the
        // endpoint's response.
        let tree = wait_for_text(&mut client, "got:").await;
        assert!(
            tree.contains("got:notes.csv:text/csv:id,name"),
            "got {tree}"
        );
    }

    #[tokio::test]
    async fn test_upload_over_max_bytes_is_rejected_while_it_streams() {
        // Small enough that Content-Length stays inside the envelope allowance, so
        // the rejection has to come from the chunk loop.
        let addr = serve_on_loopback(upload_router(UploadConstraints::new().max_bytes(8))).await;
        let mut client = connect(addr, "").await;
        let url = next_upload_url(&mut client).await;

        let response =
            post_multipart(addr, &url, "file", "big.bin", "text/plain", &[b'x'; 64]).await;

        assert!(response.contains("413"), "got {response}");
        let tree = wait_for_text(&mut client, "status:Error(").await;
        assert!(tree.contains("over the 8 byte limit"), "got {tree}");
    }

    #[tokio::test]
    async fn test_upload_far_over_max_bytes_is_rejected_from_content_length() {
        let addr = serve_on_loopback(upload_router(UploadConstraints::new().max_bytes(8))).await;
        let mut client = connect(addr, "").await;
        let url = next_upload_url(&mut client).await;

        // Past the envelope allowance, so it never gets read at all.
        let response = post_multipart(
            addr,
            &url,
            "file",
            "huge.bin",
            "text/plain",
            &[b'x'; 20 * 1024],
        )
        .await;

        assert!(response.contains("413"), "got {response}");
        let tree = wait_for_text(&mut client, "status:Error(").await;
        assert!(tree.contains("byte limit"), "got {tree}");
    }

    #[tokio::test]
    async fn test_upload_of_a_disallowed_mime_type_is_rejected() {
        let router = upload_router(UploadConstraints::new().accept(["text/csv"]));
        let addr = serve_on_loopback(router).await;
        let mut client = connect(addr, "").await;
        let url = next_upload_url(&mut client).await;

        let response = post_multipart(addr, &url, "file", "cat.png", "image/png", b"\x89PNG").await;

        assert!(response.contains("415"), "got {response}");
        let tree = wait_for_text(&mut client, "status:Error(").await;
        assert!(
            tree.contains("image/png is not one of text/csv"),
            "got {tree}"
        );
    }

    #[tokio::test]
    async fn test_upload_to_an_unknown_upload_id_is_not_found() {
        let addr = serve_on_loopback(upload_router(UploadConstraints::new())).await;
        let mut client = connect(addr, "").await;
        let url = next_upload_url(&mut client).await;

        // Same session, a slot that was never registered.
        let (prefix, _) = url.rsplit_once('/').unwrap();
        let path = format!("{prefix}/{}", Uuid::new_v4());
        let response = post_multipart(addr, &path, "file", "a.csv", "text/csv", b"hi").await;

        assert!(response.contains("404"), "got {response}");
    }

    #[tokio::test]
    async fn test_upload_to_another_connections_url_is_not_found() {
        let addr = serve_on_loopback(upload_router(UploadConstraints::new())).await;
        let mut client = connect(addr, "").await;
        let url = next_upload_url(&mut client).await;

        // The slot id is real; only the connection is wrong. Guessing an id must not
        // be enough to upload into someone else's session.
        let upload_id = url.rsplit('/').next().unwrap();
        let path = format!("/rusty/upload/{}/{upload_id}", Uuid::new_v4());
        let response = post_multipart(addr, &path, "file", "a.csv", "text/csv", b"hi").await;

        assert!(response.contains("404"), "got {response}");
    }

    #[tokio::test]
    async fn test_upload_without_a_file_field_is_a_bad_request() {
        let addr = serve_on_loopback(upload_router(UploadConstraints::new())).await;
        let mut client = connect(addr, "").await;
        let url = next_upload_url(&mut client).await;

        let response =
            post_multipart(addr, &url, "notTheFile", "a.csv", "text/csv", b"id,name").await;

        assert!(response.contains("400"), "got {response}");
        let tree = wait_for_text(&mut client, "status:Error(").await;
        assert!(tree.contains("no file field"), "got {tree}");
    }
}
