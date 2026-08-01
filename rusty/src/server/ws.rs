use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Query, State},
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

use crate::core::apps::{AppFactory, AppIds, AppRegistry};
use crate::core::runtime::RuntimeMessage;
use crate::core::services::ServiceRegistry;
use crate::views::view::View;

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
    pub session_store: AppSessionStore,
}

/// Query parameters accepted on the WebSocket upgrade.
#[derive(Debug, Default, Deserialize)]
pub struct ConnectParams {
    /// The app to mount for this connection. Sent by the frontend as `appId`.
    #[serde(rename = "appId")]
    pub app_id: Option<String>,
}

/// The Rusty WebSocket server for frontend communication.
pub struct RustyServer {
    port: u16,
    apps: AppRegistry,
    services: ServiceRegistry,
    static_dir: Option<PathBuf>,
}

impl RustyServer {
    /// Create a server serving a single root view, registered under [`AppIds::DEFAULT`].
    pub fn new<F, V>(port: u16, root_factory: F) -> Self
    where
        F: Fn() -> V + Send + Sync + 'static,
        V: View,
    {
        let mut apps = AppRegistry::new();
        apps.register(
            AppIds::DEFAULT,
            "Default",
            Arc::new(move || Box::new(root_factory()) as Box<dyn View>) as AppFactory,
        );

        RustyServer {
            port,
            apps,
            services: ServiceRegistry::new(),
            static_dir: None,
        }
    }

    /// Create a server with no apps registered. Add them with [`Self::with_app`].
    pub fn empty(port: u16) -> Self {
        RustyServer {
            port,
            apps: AppRegistry::new(),
            services: ServiceRegistry::new(),
            static_dir: None,
        }
    }

    /// Register an app under `id`, selectable by the `appId` query parameter on connect
    /// and by a `navigate` message afterwards. The first app registered becomes the
    /// default unless one is registered under [`AppIds::DEFAULT`].
    pub fn with_app<F, V>(mut self, id: &str, title: &str, factory: F) -> Self
    where
        F: Fn() -> V + Send + Sync + 'static,
        V: View,
    {
        self.apps.register(
            id,
            title,
            Arc::new(move || Box::new(factory()) as Box<dyn View>) as AppFactory,
        );
        self
    }

    /// Register a service resolvable from any view via `use_service`.
    /// Registering the same type twice overwrites the earlier instance.
    pub fn with_service<T: Send + Sync + 'static>(mut self, value: T) -> Self {
        self.services.register(value);
        self
    }

    /// Serve static files from the given directory at `/`.
    pub fn with_static_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.static_dir = Some(dir.into());
        self
    }

    /// Build the axum router with WebSocket support.
    pub fn router(self) -> Router {
        let session_store =
            AppSessionStore::with_apps(Arc::new(self.apps), Arc::new(self.services));
        let state = Arc::new(AppState { session_store });

        let mut router = Router::new()
            .route("/ws", get(ws_handler))
            .route("/health", get(health_handler))
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
        let addr = format!("0.0.0.0:{}", self.port);
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
        let addr = format!("0.0.0.0:{}", self.port);
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

/// Build the session's tree from scratch and send it as a full `Refresh`, resetting the
/// reconciler baseline to the tree just sent. Used for the initial render and after a
/// navigation, where the whole tree is replaced and patches would be meaningless.
async fn send_refresh(
    session_arc: &Arc<tokio::sync::RwLock<super::session::AppSession>>,
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

    // Generate a unique connection ID and create an isolated session for the
    // requested app (falling back to the default when absent or unknown).
    let connection_id = Uuid::new_v4().to_string();
    let session_arc = state
        .session_store
        .create_session_for_app(connection_id.clone(), app_id.as_deref())
        .await;
    let mut shutdown_rx = state.session_store.subscribe_shutdown();

    // Send initial render from this session's own runtime
    send_refresh(&session_arc, &mut sender).await;

    // Re-read after every navigation: swapping the runtime invalidates this sender.
    let mut event_tx = session_arc.read().await.runtime.event_sender();

    // Process incoming messages using this session's isolated runtime
    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
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

                                    // After event, get updated tree from this session's runtime
                                    let mut session = session_arc.write().await;
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
                                    // must not kill the connection — warn and hold position.
                                    if state
                                        .session_store
                                        .navigate_session(&session_arc, &app_id)
                                        .await
                                    {
                                        // The old runtime (and its event sender) is gone.
                                        event_tx =
                                            session_arc.read().await.runtime.event_sender();
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
                            }
                        }
                    }
                    Some(Ok(_)) => {} // Ignore non-text messages
                    _ => break, // Connection closed or error
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
    use crate::views::view::{BuildContext, Element};
    use crate::widgets::text::TextBlock;
    use tokio_tungstenite::tungstenite::Message as ClientWsMessage;

    struct LabelView {
        label: &'static str,
    }

    impl View for LabelView {
        fn build(&self, _ctx: &mut BuildContext) -> Element {
            Element::Widget(Box::new(TextBlock::new(self.label)))
        }
    }

    /// Serve a router on an ephemeral loopback port and return its address.
    /// `RustyServer::serve_background` binds 0.0.0.0, which is blocked in some sandboxes,
    /// so tests bind 127.0.0.1 explicitly.
    async fn serve_on_loopback(router: Router) -> SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        addr
    }

    /// A two-app server: `alpha` (default) renders "alpha-view", `beta` renders "beta-view".
    fn two_app_router() -> Router {
        RustyServer::empty(0)
            .with_app("alpha", "Alpha", || LabelView {
                label: "alpha-view",
            })
            .with_app("beta", "Beta", || LabelView { label: "beta-view" })
            .router()
    }

    /// Read messages until a `Refresh` arrives, returning its widgets as a JSON string.
    /// Fails the test rather than hanging if none arrives.
    async fn next_refresh<S>(socket: &mut S) -> String
    where
        S: StreamExt<Item = Result<ClientWsMessage, tokio_tungstenite::tungstenite::Error>> + Unpin,
    {
        let deadline = std::time::Duration::from_secs(5);
        tokio::time::timeout(deadline, async {
            while let Some(Ok(msg)) = socket.next().await {
                if let ClientWsMessage::Text(text) = msg {
                    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
                    if parsed.get("method").and_then(|m| m.as_str()) == Some("refresh") {
                        return parsed["widgets"].to_string();
                    }
                }
            }
            panic!("Socket closed before a refresh arrived");
        })
        .await
        .expect("Timed out waiting for a refresh message")
    }

    async fn connect(
        addr: SocketAddr,
        query: &str,
    ) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>
    {
        let url = format!("ws://{}/ws{}", addr, query);
        let (socket, _response) = tokio_tungstenite::connect_async(url)
            .await
            .expect("WebSocket connect should succeed");
        socket
    }

    async fn send_navigate<S>(socket: &mut S, app_id: &str)
    where
        S: SinkExt<ClientWsMessage> + Unpin,
        <S as futures::Sink<ClientWsMessage>>::Error: std::fmt::Debug,
    {
        let msg = serde_json::json!({
            "method": "navigate",
            "appId": app_id,
            "state": serde_json::Value::Null,
        });
        socket
            .send(ClientWsMessage::Text(msg.to_string().into()))
            .await
            .expect("navigate message should send");
    }

    #[tokio::test]
    async fn test_initial_connection_serves_default_app() {
        let addr = serve_on_loopback(two_app_router()).await;
        let mut socket = connect(addr, "").await;

        let widgets = next_refresh(&mut socket).await;
        assert!(
            widgets.contains("alpha-view"),
            "Expected the default app, got: {}",
            widgets
        );
    }

    #[tokio::test]
    async fn test_app_id_query_param_selects_the_app() {
        let addr = serve_on_loopback(two_app_router()).await;
        let mut socket = connect(addr, "?appId=beta").await;

        let widgets = next_refresh(&mut socket).await;
        assert!(
            widgets.contains("beta-view"),
            "Expected the beta app, got: {}",
            widgets
        );
    }

    #[tokio::test]
    async fn test_unknown_app_id_query_param_falls_back_to_default() {
        let addr = serve_on_loopback(two_app_router()).await;
        let mut socket = connect(addr, "?appId=does-not-exist").await;

        let widgets = next_refresh(&mut socket).await;
        assert!(
            widgets.contains("alpha-view"),
            "Expected fallback to the default app, got: {}",
            widgets
        );
    }

    #[tokio::test]
    async fn test_navigate_switches_app_and_sends_a_refresh() {
        let addr = serve_on_loopback(two_app_router()).await;
        let mut socket = connect(addr, "?appId=alpha").await;

        let first = next_refresh(&mut socket).await;
        assert!(first.contains("alpha-view"), "Got: {}", first);

        send_navigate(&mut socket, "beta").await;

        let second = next_refresh(&mut socket).await;
        assert!(
            second.contains("beta-view"),
            "Expected the beta app after navigation, got: {}",
            second
        );
        assert!(!second.contains("alpha-view"));
    }

    #[tokio::test]
    async fn test_navigate_back_and_forth() {
        let addr = serve_on_loopback(two_app_router()).await;
        let mut socket = connect(addr, "?appId=alpha").await;
        assert!(next_refresh(&mut socket).await.contains("alpha-view"));

        send_navigate(&mut socket, "beta").await;
        assert!(next_refresh(&mut socket).await.contains("beta-view"));

        send_navigate(&mut socket, "alpha").await;
        assert!(next_refresh(&mut socket).await.contains("alpha-view"));
    }

    #[tokio::test]
    async fn test_navigate_to_unknown_app_keeps_connection_and_tree() {
        let addr = serve_on_loopback(two_app_router()).await;
        let mut socket = connect(addr, "?appId=alpha").await;
        assert!(next_refresh(&mut socket).await.contains("alpha-view"));

        send_navigate(&mut socket, "does-not-exist").await;

        // No refresh should follow the ignored navigation.
        let quiet =
            tokio::time::timeout(std::time::Duration::from_millis(300), socket.next()).await;
        assert!(
            quiet.is_err(),
            "Unknown app id should produce no server message, got: {:?}",
            quiet
        );

        // The connection is still live and still on the original app: a valid
        // navigation afterwards still works.
        send_navigate(&mut socket, "beta").await;
        let widgets = next_refresh(&mut socket).await;
        assert!(
            widgets.contains("beta-view"),
            "Connection should still be usable, got: {}",
            widgets
        );
    }

    #[tokio::test]
    async fn test_navigation_does_not_affect_other_connections() {
        let addr = serve_on_loopback(two_app_router()).await;
        let mut socket_a = connect(addr, "?appId=alpha").await;
        let mut socket_b = connect(addr, "?appId=alpha").await;
        assert!(next_refresh(&mut socket_a).await.contains("alpha-view"));
        assert!(next_refresh(&mut socket_b).await.contains("alpha-view"));

        send_navigate(&mut socket_a, "beta").await;
        assert!(next_refresh(&mut socket_a).await.contains("beta-view"));

        // Connection B never asked to navigate, so it must receive nothing.
        let quiet =
            tokio::time::timeout(std::time::Duration::from_millis(300), socket_b.next()).await;
        assert!(
            quiet.is_err(),
            "Connection B should be unaffected, got: {:?}",
            quiet
        );
    }

    #[tokio::test]
    async fn test_legacy_new_constructor_still_serves_its_root_view() {
        // RustyServer::new is the compatibility hinge — it must keep working unchanged.
        let router = RustyServer::new(0, || LabelView { label: "root-view" }).router();
        let addr = serve_on_loopback(router).await;
        let mut socket = connect(addr, "").await;

        let widgets = next_refresh(&mut socket).await;
        assert!(
            widgets.contains("root-view"),
            "Expected the root view, got: {}",
            widgets
        );
    }

    #[tokio::test]
    async fn test_services_resolve_over_a_live_connection() {
        use crate::hooks::use_service::use_service;

        struct Config {
            name: &'static str,
        }

        struct ConfigView;
        impl View for ConfigView {
            fn build(&self, ctx: &mut BuildContext) -> Element {
                let config = use_service::<Config>(ctx);
                Element::Widget(Box::new(TextBlock::new(config.name)))
            }
        }

        let router = RustyServer::empty(0)
            .with_service(Config {
                name: "wired-service",
            })
            .with_app("alpha", "Alpha", || ConfigView)
            .router();
        let addr = serve_on_loopback(router).await;
        let mut socket = connect(addr, "").await;

        let widgets = next_refresh(&mut socket).await;
        assert!(
            widgets.contains("wired-service"),
            "Expected the injected service value, got: {}",
            widgets
        );
    }

    #[tokio::test]
    async fn test_health_endpoint_still_responds() {
        let addr = serve_on_loopback(two_app_router()).await;
        // A plain TCP read of the health route confirms the router is otherwise intact.
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        stream
            .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("200 OK"), "Got: {}", response);
        assert!(response.ends_with("ok"), "Got: {}", response);
    }
}
