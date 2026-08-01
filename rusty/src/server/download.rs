use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};

use uuid::Uuid;

use crate::core::query_cache::QueryError;

/// Produces the bytes for a download, called when the client requests the URL.
///
/// Deferring the work to request time means a view can offer a download without
/// paying to generate it, exactly as Ivy's `Func<Task<byte[]>>` factory does.
pub type DownloadFactory = Arc<
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<Vec<u8>, QueryError>> + Send>> + Send + Sync,
>;

/// One registered download.
struct DownloadEntry {
    factory: DownloadFactory,
    mime_type: String,
    file_name: String,
}

/// Per-connection registry of downloads reachable over HTTP.
///
/// Ported from Ivy-Framework's `IDownloadService`. Downloads are scoped to a
/// connection so one session's URLs are not guessable from another.
pub struct DownloadService {
    connection_id: String,
    entries: Mutex<HashMap<Uuid, DownloadEntry>>,
}

/// Removes its download on drop, replacing Ivy's cleanup disposable.
pub struct DownloadHandle {
    service: Weak<DownloadService>,
    download_id: Uuid,
}

impl DownloadHandle {
    pub fn download_id(&self) -> Uuid {
        self.download_id
    }
}

impl Drop for DownloadHandle {
    fn drop(&mut self) {
        if let Some(service) = self.service.upgrade() {
            service.remove(self.download_id);
        }
    }
}

impl std::fmt::Debug for DownloadHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DownloadHandle")
            .field("download_id", &self.download_id)
            .finish()
    }
}

impl DownloadService {
    pub fn new(connection_id: impl Into<String>) -> Self {
        DownloadService {
            connection_id: connection_id.into(),
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }

    /// Register a download and return its handle plus the URL to serve it from.
    /// The download stays available until the handle is dropped.
    pub fn add_download(
        self: &Arc<Self>,
        factory: DownloadFactory,
        mime_type: impl Into<String>,
        file_name: impl Into<String>,
    ) -> (DownloadHandle, String) {
        let download_id = Uuid::new_v4();
        self.entries.lock().unwrap().insert(
            download_id,
            DownloadEntry {
                factory,
                mime_type: mime_type.into(),
                file_name: file_name.into(),
            },
        );

        let url = format!("/rusty/download/{}/{}", self.connection_id, download_id);
        let handle = DownloadHandle {
            service: Arc::downgrade(self),
            download_id,
        };
        (handle, url)
    }

    /// Run a download's factory and return its bytes with the response metadata.
    ///
    /// The entry stays registered, so a download can be fetched more than once —
    /// the browser may retry, and Ivy's cleanup is tied to the view's lifetime
    /// rather than to a single request.
    pub async fn take_bytes(&self, download_id: Uuid) -> Option<DownloadResponse> {
        // The lock is released before awaiting the factory.
        let (factory, mime_type, file_name) = {
            let entries = self.entries.lock().unwrap();
            let entry = entries.get(&download_id)?;
            (
                Arc::clone(&entry.factory),
                entry.mime_type.clone(),
                entry.file_name.clone(),
            )
        };

        match (factory)().await {
            Ok(bytes) => Some(DownloadResponse {
                bytes,
                mime_type,
                file_name,
            }),
            Err(error) => {
                tracing::error!(%download_id, %error, "download factory failed");
                None
            }
        }
    }

    /// Number of downloads currently registered.
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn remove(&self, download_id: Uuid) {
        self.entries.lock().unwrap().remove(&download_id);
    }
}

impl std::fmt::Debug for DownloadService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DownloadService")
            .field("connection_id", &self.connection_id)
            .field("downloads", &self.len())
            .finish()
    }
}

/// The bytes and headers for one served download.
#[derive(Debug, Clone)]
pub struct DownloadResponse {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub file_name: String,
}

/// Wrap a typed async byte producer into a [`DownloadFactory`].
pub fn download_factory<F, Fut>(factory: F) -> DownloadFactory
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Vec<u8>, QueryError>> + Send + 'static,
{
    Arc::new(move || {
        let fut = factory();
        Box::pin(fut)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_add_download_returns_a_connection_scoped_url() {
        let service = Arc::new(DownloadService::new("conn-1"));
        let (_handle, url) = service.add_download(
            download_factory(|| async { Ok(b"data".to_vec()) }),
            "text/plain",
            "notes.txt",
        );

        assert!(
            url.starts_with("/rusty/download/conn-1/"),
            "unexpected url: {url}"
        );
        // The trailing segment is a parseable download id.
        let id = url.rsplit('/').next().unwrap();
        assert!(Uuid::parse_str(id).is_ok(), "not a uuid: {id}");
        assert_eq!(service.len(), 1);
    }

    #[tokio::test]
    async fn test_take_bytes_runs_the_factory_lazily() {
        let service = Arc::new(DownloadService::new("conn-1"));
        let calls = Arc::new(AtomicUsize::new(0));

        let factory = {
            let calls = calls.clone();
            download_factory(move || {
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(b"generated".to_vec())
                }
            })
        };
        let (handle, _url) = service.add_download(factory, "application/pdf", "report.pdf");
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "registering must not generate the bytes"
        );

        let response = service.take_bytes(handle.download_id()).await.unwrap();
        assert_eq!(response.bytes, b"generated".to_vec());
        assert_eq!(response.mime_type, "application/pdf");
        assert_eq!(response.file_name, "report.pdf");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_dropping_the_handle_unregisters_the_download() {
        let service = Arc::new(DownloadService::new("conn-1"));
        let (handle, _url) = service.add_download(
            download_factory(|| async { Ok(b"x".to_vec()) }),
            "text/plain",
            "x.txt",
        );
        let id = handle.download_id();
        assert_eq!(service.len(), 1);

        drop(handle);
        assert_eq!(service.len(), 0);
        assert!(
            service.take_bytes(id).await.is_none(),
            "an unregistered download must not be served"
        );
    }

    #[tokio::test]
    async fn test_unknown_id_and_failing_factory_both_yield_none() {
        let service = Arc::new(DownloadService::new("conn-1"));

        assert!(service.take_bytes(Uuid::new_v4()).await.is_none());

        let (handle, _url) = service.add_download(
            download_factory(|| async { Err(QueryError::new("generation failed")) }),
            "text/plain",
            "bad.txt",
        );
        assert!(service.take_bytes(handle.download_id()).await.is_none());
    }
}
