use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::core::apps::{AppFactory, AppIds, AppRegistry};
use crate::core::reconciler::Reconciler;
use crate::core::runtime::Runtime;
use crate::core::services::ServiceRegistry;

use super::ws::FuncView;

/// Per-connection state holding an isolated Runtime and Reconciler.
pub struct AppSession {
    pub runtime: Runtime,
    pub reconciler: Reconciler,
    /// The id of the app currently mounted in this session.
    pub app_id: String,
}

/// Manages per-connection AppSessions, keyed by connection ID.
/// Stores `Arc<RwLock<AppSession>>` references so both the store and handlers
/// share ownership, enabling admin/monitoring, graceful shutdown, and timeout enforcement.
pub struct AppSessionStore {
    sessions: RwLock<HashMap<String, Arc<RwLock<AppSession>>>>,
    apps: Arc<AppRegistry>,
    services: Arc<ServiceRegistry>,
    shutdown_tx: broadcast::Sender<()>,
}

impl AppSessionStore {
    /// Create a store serving a single root view under [`AppIds::DEFAULT`], with no services.
    pub fn new(root_factory: AppFactory) -> Self {
        let mut apps = AppRegistry::new();
        apps.register(AppIds::DEFAULT, "Default", root_factory);
        AppSessionStore::with_apps(Arc::new(apps), Arc::new(ServiceRegistry::new()))
    }

    /// Create a store serving an app registry, with server-level services available
    /// to every view built in every session.
    pub fn with_apps(apps: Arc<AppRegistry>, services: Arc<ServiceRegistry>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(16);
        AppSessionStore {
            sessions: RwLock::new(HashMap::new()),
            apps,
            services,
            shutdown_tx,
        }
    }

    /// The app registry backing this store.
    pub fn apps(&self) -> &Arc<AppRegistry> {
        &self.apps
    }

    /// Create a new session running the default app.
    /// Registers the connection and returns an Arc reference to the session.
    pub async fn create_session(&self, connection_id: String) -> Arc<RwLock<AppSession>> {
        self.create_session_for_app(connection_id, None).await
    }

    /// Create a new session running the requested app, falling back to the default when
    /// `app_id` is `None` or names an unknown app (see [`AppRegistry::resolve`]).
    pub async fn create_session_for_app(
        &self,
        connection_id: String,
        app_id: Option<&str>,
    ) -> Arc<RwLock<AppSession>> {
        let session = Arc::new(RwLock::new(self.build_session(app_id)));

        let mut sessions = self.sessions.write().await;
        sessions.insert(connection_id, session.clone());

        session
    }

    /// Build a fresh session for the requested app without registering it.
    /// Used both for new connections and to swap the mounted app on navigation.
    pub fn build_session(&self, app_id: Option<&str>) -> AppSession {
        let (resolved_id, view) = match self.apps.resolve(app_id) {
            Some(descriptor) => (descriptor.id.clone(), descriptor.create_view()),
            None => (
                AppIds::ERROR_NOT_FOUND.to_string(),
                Box::new(|_ctx: &mut crate::views::view::BuildContext| {
                    crate::views::view::Element::Empty
                }) as Box<dyn crate::views::view::View>,
            ),
        };

        AppSession {
            runtime: Runtime::with_services(FuncView(view), self.services.clone()),
            reconciler: Reconciler::new(),
            app_id: resolved_id,
        }
    }

    /// Replace the app mounted in an existing session, resetting its Runtime and
    /// Reconciler. Returns `false` (leaving the session untouched) when `app_id` names
    /// an app that is not registered — an explicit navigation to a typo must hold
    /// position rather than silently redirect to the default app.
    pub async fn navigate_session(&self, session: &Arc<RwLock<AppSession>>, app_id: &str) -> bool {
        if self.apps.get(app_id).is_none() {
            return false;
        }

        let fresh = self.build_session(Some(app_id));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::view::{BuildContext, Element, View};
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

    // --- App routing ---

    fn label_factory(label: &'static str) -> AppFactory {
        Arc::new(move || Box::new(TestView::new(label)))
    }

    /// A store with apps `alpha` (default) and `beta`, and no services.
    fn two_app_store() -> AppSessionStore {
        let mut apps = AppRegistry::new();
        apps.register("alpha", "Alpha", label_factory("alpha-view"));
        apps.register("beta", "Beta", label_factory("beta-view"));
        AppSessionStore::with_apps(Arc::new(apps), Arc::new(ServiceRegistry::new()))
    }

    async fn session_json(session: &Arc<RwLock<AppSession>>) -> String {
        let tree = session.write().await.runtime.build().await;
        serde_json::to_value(&tree).unwrap().to_string()
    }

    #[tokio::test]
    async fn test_new_registers_root_under_default_app_id() {
        let store = AppSessionStore::new(label_factory("root-view"));

        assert_eq!(store.apps().ids(), vec![AppIds::DEFAULT]);
        assert_eq!(store.apps().default_id(), Some(AppIds::DEFAULT));

        let session = store.create_session("conn-1".to_string()).await;
        assert_eq!(session.read().await.app_id, AppIds::DEFAULT);
        assert!(session_json(&session).await.contains("root-view"));
    }

    #[tokio::test]
    async fn test_create_session_with_explicit_app_id_builds_that_app() {
        let store = two_app_store();

        let session = store
            .create_session_for_app("conn-1".to_string(), Some("beta"))
            .await;

        assert_eq!(session.read().await.app_id, "beta");
        let json = session_json(&session).await;
        assert!(
            json.contains("beta-view"),
            "Expected beta app, got: {}",
            json
        );
    }

    #[tokio::test]
    async fn test_create_session_without_app_id_builds_default() {
        let store = two_app_store();

        let session = store.create_session("conn-1".to_string()).await;

        assert_eq!(session.read().await.app_id, "alpha");
        assert!(session_json(&session).await.contains("alpha-view"));
    }

    #[tokio::test]
    async fn test_create_session_with_unknown_app_id_falls_back_to_default() {
        let store = two_app_store();

        let session = store
            .create_session_for_app("conn-1".to_string(), Some("nope"))
            .await;

        assert_eq!(session.read().await.app_id, "alpha");
        assert!(session_json(&session).await.contains("alpha-view"));
    }

    #[tokio::test]
    async fn test_sessions_on_different_apps_get_different_trees() {
        let store = two_app_store();

        let session_a = store
            .create_session_for_app("conn-a".to_string(), Some("alpha"))
            .await;
        let session_b = store
            .create_session_for_app("conn-b".to_string(), Some("beta"))
            .await;

        let json_a = session_json(&session_a).await;
        let json_b = session_json(&session_b).await;

        assert!(json_a.contains("alpha-view"), "Got: {}", json_a);
        assert!(json_b.contains("beta-view"), "Got: {}", json_b);
        assert_ne!(json_a, json_b);
        assert_eq!(store.session_count().await, 2);
    }

    #[tokio::test]
    async fn test_navigate_session_swaps_the_mounted_app() {
        let store = two_app_store();
        let session = store.create_session("conn-1".to_string()).await;
        assert!(session_json(&session).await.contains("alpha-view"));

        let navigated = store.navigate_session(&session, "beta").await;

        assert!(navigated);
        assert_eq!(session.read().await.app_id, "beta");
        let json = session_json(&session).await;
        assert!(
            json.contains("beta-view"),
            "Expected beta app, got: {}",
            json
        );
        assert!(!json.contains("alpha-view"));
    }

    #[tokio::test]
    async fn test_navigate_session_resets_the_reconciler() {
        let store = two_app_store();
        let session = store.create_session("conn-1".to_string()).await;

        // Establish a reconciler baseline on the initial app.
        {
            let mut guard = session.write().await;
            guard.runtime.build().await;
            let tree = guard.runtime.current_tree().await.unwrap();
            guard.reconciler.reconcile(&tree);
        }

        assert!(store.navigate_session(&session, "beta").await);

        // A fresh reconciler has no baseline, so the first reconcile after navigation
        // returns None rather than patches against the old app's tree.
        let mut guard = session.write().await;
        guard.runtime.build().await;
        let tree = guard.runtime.current_tree().await.unwrap();
        assert!(
            guard.reconciler.reconcile(&tree).is_none(),
            "Reconciler should have been reset to a fresh baseline"
        );
    }

    #[tokio::test]
    async fn test_navigate_session_to_unknown_app_holds_position() {
        let store = two_app_store();
        let session = store.create_session("conn-1".to_string()).await;

        let navigated = store.navigate_session(&session, "does-not-exist").await;

        assert!(!navigated, "Navigation to an unknown id must be refused");
        assert_eq!(session.read().await.app_id, "alpha");
        let json = session_json(&session).await;
        assert!(
            json.contains("alpha-view"),
            "Session should still show the original app, got: {}",
            json
        );
    }

    #[tokio::test]
    async fn test_navigate_session_to_the_same_app_rebuilds_it() {
        let store = two_app_store();
        let session = store.create_session("conn-1".to_string()).await;

        assert!(store.navigate_session(&session, "alpha").await);
        assert_eq!(session.read().await.app_id, "alpha");
        assert!(session_json(&session).await.contains("alpha-view"));
    }

    #[tokio::test]
    async fn test_navigation_isolates_sessions() {
        let store = two_app_store();
        let session_a = store.create_session("conn-a".to_string()).await;
        let session_b = store.create_session("conn-b".to_string()).await;

        assert!(store.navigate_session(&session_a, "beta").await);

        // Only session A moved.
        assert_eq!(session_a.read().await.app_id, "beta");
        assert_eq!(session_b.read().await.app_id, "alpha");
        assert!(session_json(&session_a).await.contains("beta-view"));
        assert!(session_json(&session_b).await.contains("alpha-view"));
    }

    #[tokio::test]
    async fn test_empty_registry_yields_error_not_found_session() {
        let store = AppSessionStore::with_apps(
            Arc::new(AppRegistry::new()),
            Arc::new(ServiceRegistry::new()),
        );

        let session = store.create_session("conn-1".to_string()).await;

        assert_eq!(session.read().await.app_id, AppIds::ERROR_NOT_FOUND);
        // The placeholder view produces an empty element rather than panicking.
        let json = session_json(&session).await;
        assert!(
            json.contains("empty"),
            "Expected empty element, got: {}",
            json
        );
    }

    // --- Service injection through sessions ---

    #[tokio::test]
    async fn test_session_views_resolve_registered_services() {
        use crate::hooks::use_service::use_service;

        struct Config {
            name: String,
        }

        struct ConfigView;
        impl View for ConfigView {
            fn build(&self, ctx: &mut BuildContext) -> Element {
                let config = use_service::<Config>(ctx);
                Element::Widget(Box::new(TextBlock::new(&config.name)))
            }
        }

        let mut apps = AppRegistry::new();
        apps.register("alpha", "Alpha", Arc::new(|| Box::new(ConfigView)));

        let mut services = ServiceRegistry::new();
        services.register(Config {
            name: "injected-config".to_string(),
        });

        let store = AppSessionStore::with_apps(Arc::new(apps), Arc::new(services));
        let session = store.create_session("conn-1".to_string()).await;

        let json = session_json(&session).await;
        assert!(
            json.contains("injected-config"),
            "Expected the service value in the built tree, got: {}",
            json
        );
    }

    #[tokio::test]
    async fn test_services_survive_navigation() {
        use crate::hooks::use_service::use_service;

        struct Config {
            name: String,
        }

        struct ConfigView {
            prefix: &'static str,
        }
        impl View for ConfigView {
            fn build(&self, ctx: &mut BuildContext) -> Element {
                let config = use_service::<Config>(ctx);
                Element::Widget(Box::new(TextBlock::new(&format!(
                    "{}:{}",
                    self.prefix, config.name
                ))))
            }
        }

        let mut apps = AppRegistry::new();
        apps.register(
            "alpha",
            "Alpha",
            Arc::new(|| Box::new(ConfigView { prefix: "a" })),
        );
        apps.register(
            "beta",
            "Beta",
            Arc::new(|| Box::new(ConfigView { prefix: "b" })),
        );

        let mut services = ServiceRegistry::new();
        services.register(Config {
            name: "shared".to_string(),
        });

        let store = AppSessionStore::with_apps(Arc::new(apps), Arc::new(services));
        let session = store.create_session("conn-1".to_string()).await;
        assert!(session_json(&session).await.contains("a:shared"));

        assert!(store.navigate_session(&session, "beta").await);

        // The swapped-in runtime carries the same registry.
        let json = session_json(&session).await;
        assert!(
            json.contains("b:shared"),
            "Services should still resolve after navigation, got: {}",
            json
        );
    }
}
