use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::core::reconciler::Reconciler;
use crate::core::runtime::Runtime;
use crate::views::view::View;

use super::ws::FuncView;

/// Per-connection state holding an isolated Runtime and Reconciler.
pub struct AppSession {
    pub runtime: Runtime,
    pub reconciler: Reconciler,
}

/// Manages per-connection AppSessions, keyed by connection ID.
/// Stores `Arc<RwLock<AppSession>>` references so both the store and handlers
/// share ownership, enabling admin/monitoring, graceful shutdown, and timeout enforcement.
pub struct AppSessionStore {
    sessions: RwLock<HashMap<String, Arc<RwLock<AppSession>>>>,
    root_factory: Arc<dyn Fn() -> Box<dyn View> + Send + Sync>,
    shutdown_tx: broadcast::Sender<()>,
}

impl AppSessionStore {
    pub fn new(root_factory: Arc<dyn Fn() -> Box<dyn View> + Send + Sync>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(16);
        AppSessionStore {
            sessions: RwLock::new(HashMap::new()),
            root_factory,
            shutdown_tx,
        }
    }

    /// Create a new session with an isolated Runtime and Reconciler.
    /// Registers the connection and returns an Arc reference to the session.
    pub async fn create_session(&self, connection_id: String) -> Arc<RwLock<AppSession>> {
        let view = (self.root_factory)();
        let runtime = Runtime::new(FuncView(view));
        let reconciler = Reconciler::new();
        let session = Arc::new(RwLock::new(AppSession {
            runtime,
            reconciler,
        }));

        let mut sessions = self.sessions.write().await;
        sessions.insert(connection_id, session.clone());

        session
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
