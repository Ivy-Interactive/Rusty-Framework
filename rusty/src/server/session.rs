use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::core::apps::{AppFactory, AppIds, AppRegistry};
use crate::core::query_cache::QueryService;
use crate::core::reconciler::Reconciler;
use crate::core::runtime::Runtime;
use crate::core::services::{AppContext, ServiceRegistry};
use crate::core::signals::{ServerSignals, SignalRegistry};
use crate::views::view::View;

use super::download::DownloadService;
use super::ws::FuncView;

/// Per-connection state holding an isolated Runtime and Reconciler.
pub struct AppSession {
    pub runtime: Runtime,
    pub reconciler: Reconciler,
    /// The services this session's views resolve through `use_service`.
    pub services: Arc<ServiceRegistry>,
    /// The id of the app currently mounted in this session.
    pub app_id: String,
}

/// Manages per-connection AppSessions, keyed by connection ID.
/// Stores `Arc<RwLock<AppSession>>` references so both the store and handlers
/// share ownership, enabling admin/monitoring, graceful shutdown, and timeout enforcement.
pub struct AppSessionStore {
    sessions: RwLock<HashMap<String, Arc<RwLock<AppSession>>>>,
    /// The apps a connection can mount, resolved by id at connect and navigate time.
    apps: Arc<AppRegistry>,
    /// Registered once on `RustyServer`, folded into every session's own registry.
    server_services: Arc<ServiceRegistry>,
    shutdown_tx: broadcast::Sender<()>,
    /// Shared by every session, so a server-scoped query is fetched once.
    query_service: Arc<QueryService>,
    /// Shared by every session, so a server-scoped signal reaches all connections.
    server_signals: Arc<SignalRegistry>,
    /// Background eviction and refresh tickers for `query_service`.
    query_tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl AppSessionStore {
    /// Single-app store: `root_factory` is registered under [`AppIds::DEFAULT`], so
    /// every connection mounts it whatever `?appId=` asks for.
    pub fn new(root_factory: AppFactory) -> Self {
        let mut apps = AppRegistry::new();
        apps.register(AppIds::DEFAULT, "App", root_factory);
        Self::with_apps(Arc::new(apps), Arc::new(ServiceRegistry::new()))
    }

    /// Multi-app store: connections resolve an app out of `apps`, and every session's
    /// registry starts from `server_services`.
    pub fn with_apps(apps: Arc<AppRegistry>, server_services: Arc<ServiceRegistry>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(16);
        let query_service = Arc::new(QueryService::new());
        // Only spawn the tickers inside a runtime; `AppSessionStore::new` is also
        // called from sync contexts (`RustyServer::router`) in tests.
        let query_tasks = if tokio::runtime::Handle::try_current().is_ok() {
            query_service.start_background_tasks()
        } else {
            Vec::new()
        };

        AppSessionStore {
            sessions: RwLock::new(HashMap::new()),
            apps,
            server_services,
            shutdown_tx,
            query_service,
            server_signals: Arc::new(SignalRegistry::new()),
            query_tasks,
        }
    }

    /// The apps this store can mount.
    pub fn apps(&self) -> &Arc<AppRegistry> {
        &self.apps
    }

    /// The server-wide query cache, shared by every session.
    pub fn query_service(&self) -> &Arc<QueryService> {
        &self.query_service
    }

    /// The server-wide signal registry, shared by every session.
    pub fn server_signals(&self) -> &Arc<SignalRegistry> {
        &self.server_signals
    }

    /// Build a session mounting `app_id`, without registering it in the store.
    ///
    /// Shared by new connections and by navigation, which replaces a live session's
    /// contents in place.
    pub fn build_session(&self, connection_id: &str, app_id: Option<&str>) -> AppSession {
        let services = Arc::new(ServiceRegistry::new());
        // Server-level services first, so the framework's per-connection services below
        // always win: a `with_service::<AppContext>` must not be able to hand a session
        // another connection's id (and with it, another connection's download URLs).
        services.extend_from(&self.server_services);
        services.register(Arc::new(AppContext::new(connection_id.to_string())));
        // Server-wide, shared across connections.
        services.register(Arc::clone(&self.query_service));
        services.register(Arc::new(ServerSignals::new(Arc::clone(
            &self.server_signals,
        ))));
        // Per-connection.
        services.register(Arc::new(SignalRegistry::new()));
        services.register(Arc::new(DownloadService::new(connection_id.to_string())));

        let (resolved_id, view) = match self.apps.resolve(app_id) {
            Some(descriptor) => (descriptor.id.clone(), descriptor.create_view()),
            None => (
                AppIds::ERROR_NOT_FOUND.to_string(),
                Box::new(|_ctx: &mut crate::views::view::BuildContext| {
                    crate::views::view::Element::Empty
                }) as Box<dyn View>,
            ),
        };

        AppSession {
            runtime: Runtime::with_services(FuncView(view), Arc::clone(&services)),
            reconciler: Reconciler::new(),
            services,
            app_id: resolved_id,
        }
    }

    /// Create a new session with an isolated Runtime, Reconciler and service registry.
    /// Registers the connection and returns an Arc reference to the session.
    pub async fn create_session(&self, connection_id: String) -> Arc<RwLock<AppSession>> {
        self.create_session_for_app(connection_id, None).await
    }

    /// Create a session mounting a specific app, falling back to the default app when
    /// `app_id` is `None` or names an app that is not registered.
    pub async fn create_session_for_app(
        &self,
        connection_id: String,
        app_id: Option<&str>,
    ) -> Arc<RwLock<AppSession>> {
        let session = Arc::new(RwLock::new(self.build_session(&connection_id, app_id)));

        let mut sessions = self.sessions.write().await;
        sessions.insert(connection_id, session.clone());

        session
    }

    /// Swap the app mounted in a live session.
    ///
    /// Returns `false` without touching the session when `app_id` is not registered — a
    /// typo must not look like a successful navigation, so this uses [`AppRegistry::get`]
    /// rather than `resolve`, which would fall back to the default app.
    pub async fn navigate_session(&self, session: &Arc<RwLock<AppSession>>, app_id: &str) -> bool {
        if self.apps.get(app_id).is_none() {
            return false;
        }

        // Reuse the connection id so download URLs and `AppContext` stay stable for the
        // lifetime of the socket.
        let connection_id = {
            let guard = session.read().await;
            guard
                .services
                .get::<AppContext>()
                .map(|ctx| ctx.connection_id.clone())
                .unwrap_or_default()
        };

        let fresh = self.build_session(&connection_id, Some(app_id));
        let mut guard = session.write().await;
        *guard = fresh;
        true
    }

    /// Remove a session on disconnect.
    pub async fn remove_session(&self, connection_id: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(connection_id);
    }

    /// Get the number of active sessions.
    pub async fn session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Get a session by connection ID (for admin/monitoring).
    pub async fn get_session(&self, connection_id: &str) -> Option<Arc<RwLock<AppSession>>> {
        let sessions = self.sessions.read().await;
        sessions.get(connection_id).cloned()
    }

    /// Get all active connection IDs (for monitoring/debug).
    pub async fn connection_ids(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
    }

    /// Subscribe to the shutdown broadcast channel.
    pub fn subscribe_shutdown(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Broadcast a shutdown signal to all subscribers.
    pub fn broadcast_shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

impl Drop for AppSessionStore {
    fn drop(&mut self) {
        // The tickers hold an Arc to the query service; without aborting them the
        // cache would outlive the store.
        for task in self.query_tasks.drain(..) {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::view::{BuildContext, Element};
    use crate::widgets::text::TextBlock;

    struct TestView {
        label: String,
    }

    impl TestView {
        fn new(label: &str) -> Self {
            TestView {
                label: label.to_string(),
            }
        }
    }

    impl View for TestView {
        fn build(&self, _ctx: &mut BuildContext) -> Element {
            Element::Widget(Box::new(TextBlock::new(&self.label)))
        }
    }

    #[tokio::test]
    async fn test_session_store_create_remove() {
        let store = AppSessionStore::new(Arc::new(|| Box::new(TestView::new("hello"))));

        store.create_session("conn-1".to_string()).await;
        assert_eq!(store.session_count().await, 1);

        store.create_session("conn-2".to_string()).await;
        assert_eq!(store.session_count().await, 2);

        store.remove_session("conn-1").await;
        assert_eq!(store.session_count().await, 1);

        store.remove_session("conn-2").await;
        assert_eq!(store.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_session_isolation() {
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let store = AppSessionStore::new(Arc::new(move || {
            let n = counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Box::new(TestView::new(&format!("session-{}", n)))
        }));

        // Create two sessions — each gets its own Runtime with a different view
        let session_a = store.create_session("conn-a".to_string()).await;
        let session_b = store.create_session("conn-b".to_string()).await;

        // Build each session's tree independently
        let tree_a = session_a.write().await.runtime.build().await;
        let tree_b = session_b.write().await.runtime.build().await;

        let json_a = serde_json::to_value(&tree_a).unwrap().to_string();
        let json_b = serde_json::to_value(&tree_b).unwrap().to_string();

        assert!(
            json_a.contains("session-0"),
            "Expected session-0 in: {}",
            json_a
        );
        assert!(
            json_b.contains("session-1"),
            "Expected session-1 in: {}",
            json_b
        );
        assert_ne!(json_a, json_b);
    }

    #[tokio::test]
    async fn test_concurrent_sessions() {
        let store = Arc::new(AppSessionStore::new(Arc::new(|| {
            Box::new(TestView::new("concurrent"))
        })));

        let mut handles = vec![];
        for i in 0..10 {
            let store = store.clone();
            let handle = tokio::spawn(async move {
                let id = format!("conn-{}", i);
                let session = store.create_session(id.clone()).await;
                // Verify we got a valid session by building its tree
                let tree = session.write().await.runtime.build().await;
                let json = serde_json::to_value(&tree).unwrap().to_string();
                assert!(json.contains("concurrent"));
                store.remove_session(&id).await;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(store.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_get_session() {
        let store = AppSessionStore::new(Arc::new(|| Box::new(TestView::new("get-test"))));

        store.create_session("conn-1".to_string()).await;

        // Should return Some for an active session
        assert!(store.get_session("conn-1").await.is_some());

        // Should return None for a non-existent session
        assert!(store.get_session("conn-999").await.is_none());

        // Should return None after removal
        store.remove_session("conn-1").await;
        assert!(store.get_session("conn-1").await.is_none());
    }

    #[tokio::test]
    async fn test_connection_ids() {
        let store = AppSessionStore::new(Arc::new(|| Box::new(TestView::new("ids-test"))));

        store.create_session("conn-a".to_string()).await;
        store.create_session("conn-b".to_string()).await;
        store.create_session("conn-c".to_string()).await;

        let mut ids = store.connection_ids().await;
        ids.sort();
        assert_eq!(ids, vec!["conn-a", "conn-b", "conn-c"]);

        store.remove_session("conn-b").await;
        let mut ids = store.connection_ids().await;
        ids.sort();
        assert_eq!(ids, vec!["conn-a", "conn-c"]);
    }

    #[tokio::test]
    async fn test_broadcast_shutdown() {
        let store = AppSessionStore::new(Arc::new(|| Box::new(TestView::new("shutdown-test"))));

        let mut rx1 = store.subscribe_shutdown();
        let mut rx2 = store.subscribe_shutdown();
        let mut rx3 = store.subscribe_shutdown();

        store.broadcast_shutdown();

        // All receivers should get the signal
        assert!(rx1.recv().await.is_ok());
        assert!(rx2.recv().await.is_ok());
        assert!(rx3.recv().await.is_ok());
    }

    #[tokio::test]
    async fn test_create_session_registers_the_expected_services() {
        let store = AppSessionStore::new(Arc::new(|| Box::new(TestView::new("services"))));
        let session_arc = store.create_session("conn-1".to_string()).await;
        let services = session_arc.read().await.services.clone();

        let app_context = services.get::<AppContext>().expect("AppContext");
        assert_eq!(app_context.connection_id, "conn-1");
        assert!(services.get::<QueryService>().is_some());
        assert!(services.get::<SignalRegistry>().is_some());
        assert!(services.get::<ServerSignals>().is_some());
        assert!(services.get::<DownloadService>().is_some());

        // The runtime hands the same registry to every BuildContext it creates.
        let runtime_services = session_arc.read().await.runtime.services().clone();
        assert!(Arc::ptr_eq(&runtime_services, &services));
    }

    #[tokio::test]
    async fn test_query_service_is_shared_but_session_signals_are_not() {
        let store = AppSessionStore::new(Arc::new(|| Box::new(TestView::new("sharing"))));
        let a = store.create_session("conn-a".to_string()).await;
        let b = store.create_session("conn-b".to_string()).await;

        let services_a = a.read().await.services.clone();
        let services_b = b.read().await.services.clone();

        let query_a = services_a.get::<QueryService>().unwrap();
        let query_b = services_b.get::<QueryService>().unwrap();
        assert!(
            Arc::ptr_eq(&query_a, &query_b),
            "the query cache is server-wide"
        );
        assert!(Arc::ptr_eq(&query_a, store.query_service()));

        let server_a = services_a.get::<ServerSignals>().unwrap().registry();
        let server_b = services_b.get::<ServerSignals>().unwrap().registry();
        assert!(
            Arc::ptr_eq(&server_a, &server_b),
            "server-scoped signals are shared"
        );

        let session_a = services_a.get::<SignalRegistry>().unwrap();
        let session_b = services_b.get::<SignalRegistry>().unwrap();
        assert!(
            !Arc::ptr_eq(&session_a, &session_b),
            "session-scoped signals must be isolated per connection"
        );
    }

    #[tokio::test]
    async fn test_download_service_is_scoped_to_its_connection() {
        let store = AppSessionStore::new(Arc::new(|| Box::new(TestView::new("downloads"))));
        let a = store.create_session("conn-a".to_string()).await;
        let b = store.create_session("conn-b".to_string()).await;

        let downloads_a = a.read().await.services.get::<DownloadService>().unwrap();
        let downloads_b = b.read().await.services.get::<DownloadService>().unwrap();

        assert_eq!(downloads_a.connection_id(), "conn-a");
        assert_eq!(downloads_b.connection_id(), "conn-b");
        assert!(!Arc::ptr_eq(&downloads_a, &downloads_b));
    }

    #[tokio::test]
    async fn test_session_arc_lifecycle() {
        let store = AppSessionStore::new(Arc::new(|| Box::new(TestView::new("lifecycle-test"))));

        let session_arc = store.create_session("conn-1".to_string()).await;

        // Handler holds a clone — simulates what handle_socket does
        let handler_clone = session_arc.clone();

        // Remove from store — store's reference is dropped
        store.remove_session("conn-1").await;
        assert!(store.get_session("conn-1").await.is_none());

        // Handler's clone is still valid and usable
        let tree = handler_clone.write().await.runtime.build().await;
        let json = serde_json::to_value(&tree).unwrap().to_string();
        assert!(json.contains("lifecycle-test"));
    }
}
