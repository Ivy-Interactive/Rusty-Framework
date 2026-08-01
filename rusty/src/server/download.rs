use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};

use bytes::Bytes;
use futures::Stream;
use uuid::Uuid;

use crate::core::query_cache::QueryError;

/// Produces the bytes for a download, called when the client requests the URL.
///
/// Deferring the work to request time means a view can offer a download without
/// paying to generate it, exactly as Ivy's `Func<Task<byte[]>>` factory does.
pub type DownloadFactory = Arc<
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<Vec<u8>, QueryError>> + Send>> + Send + Sync,
>;

/// A stream of bytes for a chunked download.
pub type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, QueryError>> + Send>>;

/// Produces a stream for a download, called when the client requests the URL.
///
/// The factory returns the stream handle, so opening a file is deferred to request
/// time while the chunks stay lazy.
pub type StreamFactory = Arc<
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<ByteStream, QueryError>> + Send>> + Send + Sync,
>;

/// The source of a download's content.
enum DownloadSource {
    Bytes(DownloadFactory),
    Stream(StreamFactory),
}

/// One registered download.
struct DownloadEntry {
    source: DownloadSource,
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
        self.add_entry(DownloadSource::Bytes(factory), mime_type, file_name)
    }

    /// Register a streaming download and return its handle plus the URL.
    pub fn add_stream_download(
        self: &Arc<Self>,
        factory: StreamFactory,
        mime_type: impl Into<String>,
        file_name: impl Into<String>,
    ) -> (DownloadHandle, String) {
        self.add_entry(DownloadSource::Stream(factory), mime_type, file_name)
    }

    fn add_entry(
        self: &Arc<Self>,
        source: DownloadSource,
        mime_type: impl Into<String>,
        file_name: impl Into<String>,
    ) -> (DownloadHandle, String) {
        let download_id = Uuid::new_v4();
        self.entries.lock().unwrap().insert(
            download_id,
            DownloadEntry {
                source,
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

    /// Run a download's factory and return its payload with the response metadata.
    ///
    /// The entry stays registered, so a download can be fetched more than once —
    /// the browser may retry, and Ivy's cleanup is tied to the view's lifetime
    /// rather than to a single request.
    pub async fn take(&self, download_id: Uuid) -> Option<DownloadResponse> {
        // The lock is released before awaiting the factory.
        let (source, mime_type, file_name) = {
            let entries = self.entries.lock().unwrap();
            let entry = entries.get(&download_id)?;
            (
                match &entry.source {
                    DownloadSource::Bytes(f) => DownloadSource::Bytes(Arc::clone(f)),
                    DownloadSource::Stream(f) => DownloadSource::Stream(Arc::clone(f)),
                },
                entry.mime_type.clone(),
                entry.file_name.clone(),
            )
        };

        let payload = match source {
            DownloadSource::Bytes(factory) => match factory().await {
                Ok(bytes) => DownloadPayload::Bytes(bytes),
                Err(error) => {
                    tracing::error!(%download_id, %error, "download factory failed");
                    return None;
                }
            },
            DownloadSource::Stream(factory) => match factory().await {
                Ok(stream) => DownloadPayload::Stream(stream),
                Err(error) => {
                    tracing::error!(%download_id, %error, "stream factory failed to open");
                    return None;
                }
            },
        };

        Some(DownloadResponse {
            payload,
            mime_type,
            file_name,
        })
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

/// The payload and headers for one served download.
#[derive(Debug)]
pub struct DownloadResponse {
    pub payload: DownloadPayload,
    pub mime_type: String,
    pub file_name: String,
}

/// The content of a download — either buffered bytes or a lazy stream.
pub enum DownloadPayload {
    Bytes(Vec<u8>),
    Stream(ByteStream),
}

impl DownloadPayload {
    /// Collect the payload into a `Vec<u8>`, draining the stream if needed.
    ///
    /// For tests and small consumers that want the whole body in memory.
    pub async fn collect(self) -> Result<Vec<u8>, QueryError> {
        match self {
            DownloadPayload::Bytes(bytes) => Ok(bytes),
            DownloadPayload::Stream(mut stream) => {
                use futures::StreamExt;
                let mut buf = Vec::new();
                while let Some(chunk) = stream.next().await {
                    buf.extend_from_slice(&chunk?);
                }
                Ok(buf)
            }
        }
    }
}

impl std::fmt::Debug for DownloadPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadPayload::Bytes(bytes) => {
                f.debug_struct("Bytes").field("len", &bytes.len()).finish()
            }
            DownloadPayload::Stream(_) => f.debug_struct("Stream").finish(),
        }
    }
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

/// Wrap a typed async stream producer into a [`StreamFactory`].
pub fn stream_download_factory<F, Fut, S>(factory: F) -> StreamFactory
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<S, QueryError>> + Send + 'static,
    S: Stream<Item = Result<Bytes, QueryError>> + Send + 'static,
{
    Arc::new(move || {
        let fut = factory();
        Box::pin(async move {
            let stream = fut.await?;
            Ok(Box::pin(stream) as ByteStream)
        })
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

        let response = service.take(handle.download_id()).await.unwrap();
        assert_eq!(
            response.payload.collect().await.unwrap(),
            b"generated".to_vec()
        );
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
            service.take(id).await.is_none(),
            "an unregistered download must not be served"
        );
    }

    #[tokio::test]
    async fn test_unknown_id_and_failing_factory_both_yield_none() {
        let service = Arc::new(DownloadService::new("conn-1"));

        assert!(service.take(Uuid::new_v4()).await.is_none());

        let (handle, _url) = service.add_download(
            download_factory(|| async { Err(QueryError::new("generation failed")) }),
            "text/plain",
            "bad.txt",
        );
        assert!(service.take(handle.download_id()).await.is_none());
    }

    #[tokio::test]
    async fn test_stream_download_serves_its_chunks_in_order() {
        let service = Arc::new(DownloadService::new("conn-1"));
        let (_handle, url) = service.add_stream_download(
            stream_download_factory(|| async {
                Ok(futures::stream::iter(vec![
                    Ok(Bytes::from("id,name\n")),
                    Ok(Bytes::from("1,alice")),
                ]))
            }),
            "text/csv",
            "export.csv",
        );

        assert!(
            url.starts_with("/rusty/download/conn-1/"),
            "unexpected url: {url}"
        );

        let id = url.rsplit('/').next().unwrap();
        let download_id = Uuid::parse_str(id).unwrap();
        let response = service.take(download_id).await.unwrap();

        assert_eq!(response.mime_type, "text/csv");
        assert_eq!(response.file_name, "export.csv");
        assert_eq!(
            response.payload.collect().await.unwrap(),
            b"id,name\n1,alice".to_vec()
        );
    }

    #[tokio::test]
    async fn test_stream_factory_is_lazy_and_its_handle_unregisters() {
        let service = Arc::new(DownloadService::new("conn-1"));
        let calls = Arc::new(AtomicUsize::new(0));

        let factory = {
            let calls = calls.clone();
            stream_download_factory(move || {
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(futures::stream::iter(vec![Ok(Bytes::from("chunk"))]))
                }
            })
        };
        let (handle, _url) = service.add_stream_download(factory, "text/plain", "x.txt");
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "registering must not open the stream"
        );

        let id = handle.download_id();
        let _response = service.take(id).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        drop(handle);
        assert_eq!(service.len(), 0);
        assert!(service.take(id).await.is_none());
    }

    #[tokio::test]
    async fn test_stream_factory_error_yields_none_and_chunk_error_surfaces_on_collect() {
        let service = Arc::new(DownloadService::new("conn-1"));

        // A factory that fails to open yields None.
        let (handle, _url) = service.add_stream_download(
            stream_download_factory(|| async { Err(QueryError::new("open failed")) }),
            "text/plain",
            "bad.txt",
        );
        assert!(service.take(handle.download_id()).await.is_none());

        // A chunk error mid-stream is invisible to take and surfaces on collect.
        let (_handle, url) = service.add_stream_download(
            stream_download_factory(|| async {
                Ok(futures::stream::iter(vec![
                    Ok(Bytes::from("good")),
                    Err(QueryError::new("chunk error")),
                ]))
            }),
            "text/plain",
            "partial.txt",
        );
        let id = Uuid::parse_str(url.rsplit('/').next().unwrap()).unwrap();
        let response = service.take(id).await.unwrap();
        assert!(
            response.payload.collect().await.is_err(),
            "chunk error should surface on collect"
        );
    }
}
