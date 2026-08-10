use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

use axum::http::StatusCode;
use bytes::Bytes;
use uuid::Uuid;

/// Default cap on an upload request body, applied to the upload route only.
///
/// axum's own `DefaultBodyLimit` is 2 MiB, which would reject nearly every real
/// upload with an opaque 413 that no [`UploadError`] explains. Override with
/// [`RustyServer::with_max_upload_bytes`](crate::server::RustyServer::with_max_upload_bytes).
pub const DEFAULT_MAX_UPLOAD_BYTES: u64 = 32 * 1024 * 1024;

/// How much larger than the file a multipart request body is allowed to be when
/// the endpoint rejects an oversize upload from `Content-Length` alone.
///
/// `Content-Length` covers the boundary lines, the part headers and the file name
/// as well as the bytes, so it is only ever an *upper* bound on the file's own
/// size. Rejecting on `Content-Length > max_bytes` would therefore turn a file of
/// exactly `max_bytes` into a 413. The allowance keeps the early rejection for the
/// case it exists for — a body far too big to be worth reading — while the exact
/// limit is still enforced chunk by chunk as the body arrives.
pub const MULTIPART_ENVELOPE_ALLOWANCE: u64 = 8 * 1024;

/// What an upload slot will accept. Every field is optional — the default
/// constraints accept any file of any size.
///
/// The browser also enforces `accept` through the file picker's own filter, but a
/// client can POST whatever it likes, so the server checks again.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UploadConstraints {
    /// Accepted MIME types and extensions: `"image/*"`, `"application/pdf"`,
    /// `".csv"`. Empty accepts everything.
    pub accept: Vec<String>,
    pub max_bytes: Option<u64>,
    pub min_bytes: Option<u64>,
}

impl UploadConstraints {
    pub fn new() -> Self {
        UploadConstraints::default()
    }

    /// Restrict the accepted types. Each pattern is a MIME type (`"image/png"`), a
    /// MIME wildcard (`"image/*"`) or a file extension (`".csv"`).
    pub fn accept<I, S>(mut self, patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.accept = patterns.into_iter().map(Into::into).collect();
        self
    }

    pub fn max_bytes(mut self, max_bytes: u64) -> Self {
        self.max_bytes = Some(max_bytes);
        self
    }

    pub fn min_bytes(mut self, min_bytes: u64) -> Self {
        self.min_bytes = Some(min_bytes);
        self
    }

    /// Whether a file with this MIME type and name passes the `accept` list.
    pub fn accepts(&self, mime_type: &str, file_name: &str) -> bool {
        accepts(&self.accept, mime_type, file_name)
    }
}

/// One file that arrived over the upload endpoint, held in memory.
#[derive(Clone, PartialEq, Eq)]
pub struct UploadedFile {
    pub file_name: String,
    pub mime_type: String,
    pub content: Bytes,
}

impl UploadedFile {
    pub fn len(&self) -> usize {
        self.content.len()
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

/// Prints the size instead of the bytes: an upload is routinely megabytes, and a
/// `{:?}` in a log line should not dump them.
impl std::fmt::Debug for UploadedFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadedFile")
            .field("file_name", &self.file_name)
            .field("mime_type", &self.mime_type)
            .field("len", &self.content.len())
            .finish()
    }
}

/// Why an upload did not produce a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadError {
    /// The request carried no `file` field.
    NoFile,
    TooLarge {
        limit: u64,
        actual: u64,
    },
    TooSmall {
        limit: u64,
        actual: u64,
    },
    RejectedMimeType {
        mime_type: String,
        accept: Vec<String>,
    },
    /// The view called `Upload::cancel` while the body was still arriving.
    Cancelled,
    /// The connection broke or the multipart body was malformed.
    Transport(String),
}

impl UploadError {
    /// The status code the endpoint answers with.
    ///
    /// All of these are 4xx: every variant is something about the *request*, even
    /// `Transport`, which here only ever means a body the server could not parse
    /// or finish reading.
    pub fn status_code(&self) -> StatusCode {
        match self {
            UploadError::TooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            UploadError::TooSmall { .. } | UploadError::RejectedMimeType { .. } => {
                StatusCode::UNSUPPORTED_MEDIA_TYPE
            }
            UploadError::NoFile | UploadError::Cancelled | UploadError::Transport(_) => {
                StatusCode::BAD_REQUEST
            }
        }
    }
}

impl std::fmt::Display for UploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UploadError::NoFile => write!(f, "the request contained no file field"),
            UploadError::TooLarge { limit, actual } => {
                write!(f, "file is {actual} bytes, over the {limit} byte limit")
            }
            UploadError::TooSmall { limit, actual } => {
                write!(f, "file is {actual} bytes, under the {limit} byte minimum")
            }
            UploadError::RejectedMimeType { mime_type, accept } => {
                write!(f, "{mime_type} is not one of {}", accept.join(", "))
            }
            UploadError::Cancelled => write!(f, "the upload was cancelled"),
            UploadError::Transport(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for UploadError {}

/// What the endpoint reports back to the view as a body arrives.
#[derive(Debug, Clone)]
pub enum UploadEvent {
    /// `total` is the request's `Content-Length` when it sent one, so it includes
    /// the multipart envelope and is an upper bound on the file's own size.
    Progress {
        received: u64,
        total: Option<u64>,
    },
    Completed(UploadedFile),
    Failed(UploadError),
}

/// Receives every [`UploadEvent`] for one slot. Called from the HTTP task, not
/// from a build.
pub type UploadObserver = Arc<dyn Fn(UploadEvent) + Send + Sync>;

/// One registered upload slot.
struct UploadEntry {
    observer: UploadObserver,
    constraints: UploadConstraints,
    cancel: Arc<AtomicBool>,
}

/// A slot resolved for one in-flight request.
///
/// [`UploadService::slot`] clones this out from under the registry lock, so the
/// request never holds the lock while reading a body.
#[derive(Clone)]
pub struct UploadSlot {
    pub constraints: UploadConstraints,
    observer: UploadObserver,
    cancel: Arc<AtomicBool>,
}

impl UploadSlot {
    /// Report an event to the view that registered the slot.
    pub fn emit(&self, event: UploadEvent) {
        (self.observer)(event);
    }

    /// Whether the view has asked for the in-flight upload to stop.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
}

impl std::fmt::Debug for UploadSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadSlot")
            .field("constraints", &self.constraints)
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}

/// Per-connection registry of upload slots reachable over HTTP.
///
/// The mirror image of [`DownloadService`](super::download::DownloadService):
/// slots are scoped to a connection so one session's upload URLs are not
/// guessable from another, and a slot lives exactly as long as its handle.
pub struct UploadService {
    connection_id: String,
    entries: Mutex<HashMap<Uuid, UploadEntry>>,
}

/// Removes its slot on drop, so a view's upload URL dies with the view.
pub struct UploadHandle {
    service: Weak<UploadService>,
    upload_id: Uuid,
    cancel: Arc<AtomicBool>,
}

impl UploadHandle {
    pub fn upload_id(&self) -> Uuid {
        self.upload_id
    }

    /// Ask an in-flight request to stop. The endpoint notices between chunks.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// The shared cancellation flag, so a hook can raise it without the handle.
    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel)
    }
}

impl Drop for UploadHandle {
    fn drop(&mut self) {
        if let Some(service) = self.service.upgrade() {
            service.remove(self.upload_id);
        }
    }
}

impl std::fmt::Debug for UploadHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadHandle")
            .field("upload_id", &self.upload_id)
            .finish()
    }
}

impl UploadService {
    pub fn new(connection_id: impl Into<String>) -> Self {
        UploadService {
            connection_id: connection_id.into(),
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }

    /// Register an upload slot and return its handle plus the URL to POST to.
    /// The slot stays available until the handle is dropped.
    pub fn add_upload(
        self: &Arc<Self>,
        observer: UploadObserver,
        constraints: UploadConstraints,
    ) -> (UploadHandle, String) {
        self.add_upload_with_cancel(observer, constraints, Arc::new(AtomicBool::new(false)))
    }

    /// [`add_upload`](Self::add_upload) with a caller-supplied cancellation flag,
    /// for a hook that must be able to cancel from a build without waiting for
    /// the mount effect to hand back a handle.
    pub fn add_upload_with_cancel(
        self: &Arc<Self>,
        observer: UploadObserver,
        constraints: UploadConstraints,
        cancel: Arc<AtomicBool>,
    ) -> (UploadHandle, String) {
        let upload_id = Uuid::new_v4();
        self.entries.lock().unwrap().insert(
            upload_id,
            UploadEntry {
                observer,
                constraints,
                cancel: Arc::clone(&cancel),
            },
        );

        let url = format!("/rusty/upload/{}/{}", self.connection_id, upload_id);
        let handle = UploadHandle {
            service: Arc::downgrade(self),
            upload_id,
            cancel,
        };
        (handle, url)
    }

    /// Resolve a slot for one request, cloning it out from under the lock.
    ///
    /// Unlike `DownloadService::take` this does **not** remove the entry: a slot is
    /// reusable across files (pick a file, then pick another), and its lifetime
    /// belongs to the handle rather than to a single request.
    pub fn slot(&self, upload_id: Uuid) -> Option<UploadSlot> {
        let entries = self.entries.lock().unwrap();
        let entry = entries.get(&upload_id)?;
        Some(UploadSlot {
            constraints: entry.constraints.clone(),
            observer: Arc::clone(&entry.observer),
            cancel: Arc::clone(&entry.cancel),
        })
    }

    /// Number of upload slots currently registered.
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn remove(&self, upload_id: Uuid) {
        self.entries.lock().unwrap().remove(&upload_id);
    }
}

impl std::fmt::Debug for UploadService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadService")
            .field("connection_id", &self.connection_id)
            .field("uploads", &self.len())
            .finish()
    }
}

/// Whether `mime_type`/`file_name` satisfies an HTML `accept` list.
///
/// An empty list accepts everything. Entries are matched case-insensitively; a
/// `type/*` entry matches on the type, a `.ext` entry on the file name's suffix,
/// and anything else on the exact MIME type with its parameters (`; charset=..`)
/// stripped.
pub fn accepts(accept: &[String], mime_type: &str, file_name: &str) -> bool {
    if accept.is_empty() {
        return true;
    }

    let base_mime = mime_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let lower_name = file_name.to_ascii_lowercase();

    accept.iter().any(|pattern| {
        let pattern = pattern.trim().to_ascii_lowercase();
        if pattern.is_empty() {
            false
        } else if pattern == "*" || pattern == "*/*" {
            true
        } else if let Some(type_prefix) = pattern.strip_suffix("/*") {
            base_mime
                .split('/')
                .next()
                .is_some_and(|actual| actual == type_prefix)
        } else if pattern.starts_with('.') {
            lower_name.ends_with(&pattern)
        } else {
            base_mime == pattern
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    /// An observer that records every event it sees.
    fn recording_observer() -> (UploadObserver, Arc<Mutex<Vec<UploadEvent>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        let observer: UploadObserver = Arc::new(move |event| sink.lock().unwrap().push(event));
        (observer, events)
    }

    fn a_file(name: &str) -> UploadedFile {
        UploadedFile {
            file_name: name.to_string(),
            mime_type: "text/csv".to_string(),
            content: Bytes::from_static(b"id,name"),
        }
    }

    #[test]
    fn test_add_upload_returns_a_connection_scoped_url() {
        let service = Arc::new(UploadService::new("conn-1"));
        let (observer, _events) = recording_observer();

        let (_handle, url) = service.add_upload(observer, UploadConstraints::new());

        assert!(
            url.starts_with("/rusty/upload/conn-1/"),
            "unexpected url: {url}"
        );
        let id = url.rsplit('/').next().unwrap();
        assert!(Uuid::parse_str(id).is_ok(), "not a uuid: {id}");
        assert_eq!(service.len(), 1);
    }

    #[test]
    fn test_dropping_the_handle_unregisters_the_slot() {
        let service = Arc::new(UploadService::new("conn-1"));
        let (observer, _events) = recording_observer();
        let (handle, _url) = service.add_upload(observer, UploadConstraints::new());
        let id = handle.upload_id();
        assert!(service.slot(id).is_some());

        drop(handle);

        assert_eq!(service.len(), 0);
        assert!(
            service.slot(id).is_none(),
            "an unregistered slot must not resolve"
        );
    }

    #[test]
    fn test_slot_does_not_remove_the_entry() {
        let service = Arc::new(UploadService::new("conn-1"));
        let (observer, events) = recording_observer();
        let (handle, _url) = service.add_upload(observer, UploadConstraints::new().max_bytes(1024));
        let id = handle.upload_id();

        // Two sequential uploads through one slot, as a view that lets the user
        // pick a second file would do.
        let first = service.slot(id).expect("first upload");
        assert_eq!(first.constraints.max_bytes, Some(1024));
        first.emit(UploadEvent::Completed(a_file("one.csv")));

        let second = service
            .slot(id)
            .expect("second upload through the same slot");
        second.emit(UploadEvent::Completed(a_file("two.csv")));

        assert_eq!(service.len(), 1, "the slot must survive a completed upload");
        assert_eq!(events.lock().unwrap().len(), 2);
    }

    #[test]
    fn test_unknown_id_does_not_resolve() {
        let service = Arc::new(UploadService::new("conn-1"));
        assert!(service.slot(Uuid::new_v4()).is_none());
        assert!(service.is_empty());
    }

    #[test]
    fn test_cancel_flag_is_shared_between_handle_and_slot() {
        let service = Arc::new(UploadService::new("conn-1"));
        let (observer, _events) = recording_observer();
        let (handle, _url) = service.add_upload(observer, UploadConstraints::new());

        let slot = service.slot(handle.upload_id()).unwrap();
        assert!(!slot.is_cancelled());

        handle.cancel();
        assert!(slot.is_cancelled(), "the flag is shared, not copied");
        // And a slot resolved after the cancel sees it too.
        assert!(service.slot(handle.upload_id()).unwrap().is_cancelled());
    }

    #[test]
    fn test_caller_supplied_cancel_flag_is_the_one_the_slot_sees() {
        let service = Arc::new(UploadService::new("conn-1"));
        let (observer, _events) = recording_observer();
        let cancel = Arc::new(AtomicBool::new(false));

        let (handle, _url) =
            service.add_upload_with_cancel(observer, UploadConstraints::new(), Arc::clone(&cancel));

        cancel.store(true, Ordering::SeqCst);
        assert!(service.slot(handle.upload_id()).unwrap().is_cancelled());
    }

    #[test]
    fn test_observer_receives_every_event() {
        let service = Arc::new(UploadService::new("conn-1"));
        let (observer, events) = recording_observer();
        let (handle, _url) = service.add_upload(observer, UploadConstraints::new());
        let slot = service.slot(handle.upload_id()).unwrap();

        slot.emit(UploadEvent::Progress {
            received: 4,
            total: Some(8),
        });
        slot.emit(UploadEvent::Failed(UploadError::NoFile));

        let events = events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            UploadEvent::Progress {
                received: 4,
                total: Some(8)
            }
        ));
        assert!(matches!(
            events[1],
            UploadEvent::Failed(UploadError::NoFile)
        ));
    }

    #[test]
    fn test_slots_are_isolated_per_service() {
        let a = Arc::new(UploadService::new("conn-a"));
        let b = Arc::new(UploadService::new("conn-b"));
        let (observer, _events) = recording_observer();
        let (handle, _url) = a.add_upload(observer, UploadConstraints::new());

        assert!(
            b.slot(handle.upload_id()).is_none(),
            "another connection must not resolve this slot"
        );
        assert_eq!(a.connection_id(), "conn-a");
        assert_eq!(b.connection_id(), "conn-b");
    }

    #[test]
    fn test_accepts_matches_wildcards_exact_types_and_extensions() {
        // (accept, mime, name, expected)
        let cases: &[(&[&str], &str, &str, bool)] = &[
            (&[], "application/x-anything", "weird.bin", true),
            (&["image/*"], "image/png", "logo.png", true),
            (&["image/*"], "text/plain", "notes.txt", false),
            (&["IMAGE/*"], "image/jpeg", "photo.jpg", true),
            (&["image/*"], "IMAGE/JPEG", "photo.jpg", true),
            (&["application/pdf"], "application/pdf", "a.pdf", true),
            (&["application/pdf"], "application/x-pdf", "a.pdf", false),
            (&["text/csv"], "text/csv; charset=utf-8", "a.csv", true),
            (&[".csv"], "application/octet-stream", "export.csv", true),
            (&[".csv"], "application/octet-stream", "export.CSV", true),
            (&[".CSV"], "application/octet-stream", "export.csv", true),
            (&[".csv"], "application/octet-stream", "export.tsv", false),
            (
                &["image/*", ".csv"],
                "application/octet-stream",
                "a.csv",
                true,
            ),
            (&["*/*"], "application/zip", "a.zip", true),
            (&["image/png"], "image/png", "", true),
        ];

        for (accept, mime, name, expected) in cases {
            let accept: Vec<String> = accept.iter().map(|s| s.to_string()).collect();
            assert_eq!(
                accepts(&accept, mime, name),
                *expected,
                "accepts({accept:?}, {mime:?}, {name:?})"
            );
        }
    }

    #[test]
    fn test_constraints_builder_carries_its_accept_list() {
        let constraints = UploadConstraints::new()
            .accept([".csv", "text/csv"])
            .max_bytes(100)
            .min_bytes(1);

        assert_eq!(constraints.max_bytes, Some(100));
        assert_eq!(constraints.min_bytes, Some(1));
        assert!(constraints.accepts("application/octet-stream", "data.csv"));
        assert!(!constraints.accepts("image/png", "logo.png"));
    }

    #[test]
    fn test_upload_error_status_codes() {
        assert_eq!(
            UploadError::TooLarge {
                limit: 1,
                actual: 2
            }
            .status_code(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(
            UploadError::TooSmall {
                limit: 2,
                actual: 1
            }
            .status_code(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        assert_eq!(
            UploadError::RejectedMimeType {
                mime_type: "image/png".into(),
                accept: vec![".csv".into()]
            }
            .status_code(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        assert_eq!(UploadError::NoFile.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(
            UploadError::Cancelled.status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            UploadError::Transport("broken".into()).status_code(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn test_upload_error_messages_name_the_numbers() {
        assert!(UploadError::TooLarge {
            limit: 10,
            actual: 99
        }
        .to_string()
        .contains("99"));
        assert!(UploadError::RejectedMimeType {
            mime_type: "image/png".into(),
            accept: vec!["text/csv".into(), ".csv".into()]
        }
        .to_string()
        .contains("text/csv, .csv"));
    }

    #[test]
    fn test_uploaded_file_debug_omits_the_bytes() {
        let file = UploadedFile {
            file_name: "secret.bin".into(),
            mime_type: "application/octet-stream".into(),
            content: Bytes::from_static(b"sensitive-payload"),
        };
        let rendered = format!("{file:?}");

        assert!(rendered.contains("secret.bin"), "got {rendered}");
        assert!(rendered.contains("len: 17"), "got {rendered}");
        assert!(!rendered.contains("sensitive"), "got {rendered}");
        assert_eq!(file.len(), 17);
        assert!(!file.is_empty());
    }

    #[test]
    fn test_observer_is_shared_not_cloned_per_slot() {
        let service = Arc::new(UploadService::new("conn-1"));
        let calls = Arc::new(AtomicUsize::new(0));
        let observer: UploadObserver = {
            let calls = Arc::clone(&calls);
            Arc::new(move |_| {
                calls.fetch_add(1, Ordering::SeqCst);
            })
        };
        let (handle, _url) = service.add_upload(observer, UploadConstraints::new());

        service
            .slot(handle.upload_id())
            .unwrap()
            .emit(UploadEvent::Failed(UploadError::Cancelled));
        service
            .slot(handle.upload_id())
            .unwrap()
            .emit(UploadEvent::Failed(UploadError::Cancelled));

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
