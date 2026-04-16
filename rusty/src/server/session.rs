use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
/// Tracks active connections and creates isolated sessions on demand.
pub struct AppSessionStore {
    sessions: RwLock<HashMap<String, ()>>,
    root_factory: Arc<dyn Fn() -> Box<dyn View> + Send + Sync>,
}

impl AppSessionStore {
    pub fn new(root_factory: Arc<dyn Fn() -> Box<dyn View> + Send + Sync>) -> Self {
        AppSessionStore {
            sessions: RwLock::new(HashMap::new()),
            root_factory,
        }
    }

    /// Create a new session with an isolated Runtime and Reconciler.
    /// Registers the connection and returns the session for the handler to own.
    pub async fn create_session(&self, connection_id: String) -> AppSession {
        let view = (self.root_factory)();
        let runtime = Runtime::new(FuncView(view));
        let reconciler = Reconciler::new();

        let mut sessions = self.sessions.write().await;
        sessions.insert(connection_id, ());

        AppSession {
            runtime,
            reconciler,
        }
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
        let tree_a = session_a.runtime.build().await;
        let tree_b = session_b.runtime.build().await;

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
                let tree = session.runtime.build().await;
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
}
