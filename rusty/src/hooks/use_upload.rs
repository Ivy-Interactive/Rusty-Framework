use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::core::query_cache::QueryError;
use crate::hooks::use_effect::use_effect;
use crate::hooks::use_ref::use_ref;
use crate::hooks::use_state::{use_state, State};
use crate::server::upload::{
    UploadConstraints, UploadEvent, UploadObserver, UploadService, UploadedFile,
};
use crate::views::view::BuildContext;

/// Where an upload slot is in its lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UploadStatus {
    /// Registered, nothing received yet.
    #[default]
    Idle,
    /// A body is arriving.
    Uploading,
    /// The file is in hand (and, for [`use_upload_to`], the sink accepted it).
    Done,
    /// The upload was rejected or broke — the string is the
    /// [`UploadError`](crate::server::upload::UploadError)'s message.
    Error(String),
}

/// The upload slot a view holds, handed back by [`use_upload`].
///
/// Cheap to clone: every field is `Arc`-backed, so an event handler can capture it.
pub struct Upload {
    /// The URL to POST the file to. `None` until the mount effect registers the
    /// slot, so a view renders its picker only when this is `Some`.
    pub url: State<Option<String>>,
    /// Bytes the **server** has received, as a percentage of the request's
    /// `Content-Length`, clamped to `99` until the file is complete — so `100`
    /// always means the bytes are in hand. The browser's own optimistic progress
    /// bar (`useUploadWithProgress.ts`) is a separate number that reaches 100 as
    /// soon as the socket is drained.
    pub progress: State<u8>,
    pub status: State<UploadStatus>,
    /// The received file. Always `None` for [`use_upload_to`], which hands the
    /// bytes to its sink instead of holding them in view state.
    pub file: State<Option<UploadedFile>>,
    cancel: Arc<dyn Fn() + Send + Sync>,
    reset: Arc<dyn Fn() + Send + Sync>,
}

impl Upload {
    /// Ask an in-flight upload to stop. The endpoint notices between chunks and
    /// answers `400`; the status becomes `Error("the upload was cancelled")`.
    ///
    /// A no-op if nothing is in flight — the flag stays raised, so call
    /// [`reset`](Self::reset) before offering the slot again.
    pub fn cancel(&self) {
        (self.cancel)();
    }

    pub fn is_uploading(&self) -> bool {
        self.status.get() == UploadStatus::Uploading
    }

    /// Back to `Idle` with no file, no progress and the cancellation flag cleared.
    /// The URL is untouched: the slot is still registered and reusable.
    pub fn reset(&self) {
        (self.reset)();
    }
}

impl Clone for Upload {
    fn clone(&self) -> Self {
        Upload {
            url: self.url.clone(),
            progress: self.progress.clone(),
            status: self.status.clone(),
            file: self.file.clone(),
            cancel: Arc::clone(&self.cancel),
            reset: Arc::clone(&self.reset),
        }
    }
}

impl std::fmt::Debug for Upload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Upload")
            .field("url", &self.url.get())
            .field("progress", &self.progress.get())
            .field("status", &self.status.get())
            .field("file", &self.file.get())
            .finish()
    }
}

/// Persists an uploaded file somewhere other than view state.
type UploadSink = Arc<
    dyn Fn(UploadedFile) -> std::pin::Pin<Box<dyn Future<Output = Result<(), QueryError>> + Send>>
        + Send
        + Sync,
>;

/// Register an upload slot and get back its URL plus the progress state.
///
/// The client half already exists: `uploadFileWithProgress` in the frontend POSTs
/// a `FormData` with a single field named `file` to this URL over XHR, so the
/// endpoint this hook registers matches that shape exactly.
///
/// The received bytes land in [`Upload::file`]. For anything large enough that
/// holding it in view state is wrong, use [`use_upload_to`].
///
/// Requires an [`UploadService`] on the registry, which `AppSessionStore` registers
/// per connection — so upload URLs are not guessable across sessions.
pub fn use_upload(ctx: &mut BuildContext, constraints: UploadConstraints) -> Upload {
    use_upload_inner(ctx, constraints, None)
}

/// [`use_upload`] that streams each completed file into `sink` instead of keeping
/// it in view state.
///
/// [`Upload::file`] stays `None`. The status reaches `Done` only once the sink's
/// future resolves `Ok`; a sink error becomes `Error` with the `QueryError`'s
/// message, so a failed database write is as visible as a rejected MIME type.
pub fn use_upload_to<F, Fut>(
    ctx: &mut BuildContext,
    constraints: UploadConstraints,
    sink: F,
) -> Upload
where
    F: Fn(UploadedFile) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), QueryError>> + Send + 'static,
{
    let sink: UploadSink = Arc::new(move |file| Box::pin(sink(file)));
    use_upload_inner(ctx, constraints, Some(sink))
}

/// Slot layout shared by both hooks, so swapping one for the other at a call site
/// does not shift any later hook's slot: `url`, `progress`, `status`, `file`,
/// `cancel flag`, `effect`.
fn use_upload_inner(
    ctx: &mut BuildContext,
    constraints: UploadConstraints,
    sink: Option<UploadSink>,
) -> Upload {
    let url = use_state(ctx, None::<String>);
    let progress = use_state(ctx, 0u8);
    let status = use_state(ctx, UploadStatus::default());
    let file = use_state(ctx, None::<UploadedFile>);
    let cancel_flag = use_ref(ctx, Arc::new(AtomicBool::new(false)));

    let service = ctx
        .services()
        .get::<UploadService>()
        .expect("use_upload requires an UploadService on the ServiceRegistry");

    let observer = observer_for(&progress, &status, &file, sink);
    let flag = cancel_flag.get();

    let effect_url = url.clone();
    let effect_flag = Arc::clone(&flag);
    use_effect(ctx, move || {
        let (handle, slot_url) = service.add_upload_with_cancel(observer, constraints, effect_flag);
        effect_url.set(Some(slot_url));

        // Dropping the handle unregisters the slot.
        Some(Box::new(move || {
            drop(handle);
        }) as Box<dyn FnOnce() + Send + Sync>)
    });

    let cancel = {
        let flag = Arc::clone(&flag);
        Arc::new(move || flag.store(true, Ordering::SeqCst)) as Arc<dyn Fn() + Send + Sync>
    };
    let reset = {
        let flag = Arc::clone(&flag);
        let progress = progress.clone();
        let status = status.clone();
        let file = file.clone();
        Arc::new(move || {
            flag.store(false, Ordering::SeqCst);
            progress.set(0);
            file.set(None);
            status.set(UploadStatus::Idle);
        }) as Arc<dyn Fn() + Send + Sync>
    };

    Upload {
        url,
        progress,
        status,
        file,
        cancel,
        reset,
    }
}

/// The observer the endpoint calls as a body arrives, writing the hook's states.
///
/// Runs on the HTTP task, not during a build, so `State::set` here is the same
/// cross-task path `use_query` uses: the `RebuildHandle` inside `State` pushes the
/// rebuild out over the WebSocket.
fn observer_for(
    progress: &State<u8>,
    status: &State<UploadStatus>,
    file: &State<Option<UploadedFile>>,
    sink: Option<UploadSink>,
) -> UploadObserver {
    let progress = progress.clone();
    let status = status.clone();
    let file = file.clone();

    Arc::new(move |event| match event {
        UploadEvent::Progress { received, total } => {
            // An unknown total leaves the percentage where it was: a bar that
            // cannot advance is better than one that snaps back to zero.
            if let Some(total) = total {
                if total > 0 {
                    progress.set(percent_received(received, total));
                }
            }
            if status.get() != UploadStatus::Uploading {
                status.set(UploadStatus::Uploading);
            }
        }
        UploadEvent::Completed(received) => match sink.clone() {
            None => {
                file.set(Some(received));
                progress.set(100);
                status.set(UploadStatus::Done);
            }
            Some(sink) => {
                let status = status.clone();
                let progress = progress.clone();
                tokio::spawn(async move {
                    match sink(received).await {
                        Ok(()) => {
                            progress.set(100);
                            status.set(UploadStatus::Done);
                        }
                        Err(error) => status.set(UploadStatus::Error(error.message)),
                    }
                });
            }
        },
        UploadEvent::Failed(error) => {
            status.set(UploadStatus::Error(error.to_string()));
        }
    })
}

/// Received bytes as a percentage, capped at 99 so only `Completed` reports 100.
fn percent_received(received: u64, total: u64) -> u8 {
    let percent = received.saturating_mul(100) / total;
    percent.min(99) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::services::ServiceRegistry;
    use crate::hooks::hook_store::HookStore;
    use crate::server::upload::{UploadError, UploadSlot};
    use crate::views::view::EffectCleanup;
    use bytes::Bytes;
    use std::sync::Mutex;
    use uuid::Uuid;

    fn test_services() -> (Arc<ServiceRegistry>, Arc<UploadService>) {
        let upload_service = Arc::new(UploadService::new("conn-1"));
        let services = Arc::new(ServiceRegistry::new());
        services.register(Arc::clone(&upload_service));
        (services, upload_service)
    }

    fn a_file() -> UploadedFile {
        UploadedFile {
            file_name: "export.csv".to_string(),
            mime_type: "text/csv".to_string(),
            content: Bytes::from_static(b"id,name\n1,alice"),
        }
    }

    /// Build once, run the effects, and return the hook's result plus any cleanups.
    fn mount(
        store: &mut HookStore,
        services: &Arc<ServiceRegistry>,
        constraints: UploadConstraints,
    ) -> (Upload, Vec<EffectCleanup>) {
        mount_with(store, services, |ctx| use_upload(ctx, constraints))
    }

    fn mount_with(
        store: &mut HookStore,
        services: &Arc<ServiceRegistry>,
        build: impl FnOnce(&mut BuildContext) -> Upload,
    ) -> (Upload, Vec<EffectCleanup>) {
        let (upload, effects) = {
            let mut ctx =
                BuildContext::with_services(store, None, Uuid::nil(), Arc::clone(services));
            let upload = build(&mut ctx);
            (upload, ctx.drain_effects())
        };

        let mut cleanups = Vec::new();
        for effect in effects {
            if let Some(cleanup) = (effect.callback)() {
                cleanups.push(cleanup);
            }
        }
        (upload, cleanups)
    }

    /// The slot the hook registered, resolved from the URL it published.
    fn slot_of(service: &Arc<UploadService>, upload: &Upload) -> UploadSlot {
        let url = upload
            .url
            .get()
            .expect("the effect should have set the url");
        let id = Uuid::parse_str(url.rsplit('/').next().unwrap()).unwrap();
        service.slot(id).expect("the slot should be registered")
    }

    /// Poll `predicate` until it holds — for the states a spawned sink task writes.
    async fn wait_until(predicate: impl Fn() -> bool) {
        for _ in 0..200 {
            if predicate() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        panic!("condition never held");
    }

    #[tokio::test]
    async fn test_mount_registers_exactly_one_slot_and_publishes_its_url() {
        let (services, service) = test_services();
        let mut store = HookStore::new();

        let (upload, _cleanups) = mount(&mut store, &services, UploadConstraints::new());

        let url = upload
            .url
            .get()
            .expect("the effect should have set the url");
        assert!(
            url.starts_with("/rusty/upload/conn-1/"),
            "unexpected url: {url}"
        );
        assert_eq!(service.len(), 1);
        assert_eq!(upload.status.get(), UploadStatus::Idle);
        assert_eq!(upload.progress.get(), 0);
        assert!(upload.file.get().is_none());
        assert!(!upload.is_uploading());
    }

    #[tokio::test]
    async fn test_constraints_reach_the_registered_slot() {
        let (services, service) = test_services();
        let mut store = HookStore::new();

        let (upload, _cleanups) = mount(
            &mut store,
            &services,
            UploadConstraints::new().accept([".csv"]).max_bytes(64),
        );

        let slot = slot_of(&service, &upload);
        assert_eq!(slot.constraints.max_bytes, Some(64));
        assert!(slot
            .constraints
            .accepts("application/octet-stream", "a.csv"));
    }

    #[tokio::test]
    async fn test_progress_events_move_the_percentage_and_completion_reaches_100() {
        let (services, service) = test_services();
        let mut store = HookStore::new();

        let (upload, _cleanups) = mount(&mut store, &services, UploadConstraints::new());
        let slot = slot_of(&service, &upload);

        slot.emit(UploadEvent::Progress {
            received: 0,
            total: Some(15),
        });
        assert_eq!(upload.progress.get(), 0);
        assert_eq!(upload.status.get(), UploadStatus::Uploading);
        assert!(upload.is_uploading());

        // Even a fully received body reports 99 while it is still a Progress event.
        slot.emit(UploadEvent::Progress {
            received: 15,
            total: Some(15),
        });
        assert_eq!(upload.progress.get(), 99);

        slot.emit(UploadEvent::Completed(a_file()));
        assert_eq!(upload.progress.get(), 100);
        assert_eq!(upload.status.get(), UploadStatus::Done);
        assert_eq!(
            upload
                .file
                .get()
                .expect("the file should be in hand")
                .content,
            Bytes::from_static(b"id,name\n1,alice")
        );
    }

    #[tokio::test]
    async fn test_progress_without_a_total_leaves_the_percentage_alone() {
        let (services, service) = test_services();
        let mut store = HookStore::new();

        let (upload, _cleanups) = mount(&mut store, &services, UploadConstraints::new());
        let slot = slot_of(&service, &upload);

        slot.emit(UploadEvent::Progress {
            received: 5,
            total: Some(10),
        });
        assert_eq!(upload.progress.get(), 50);

        slot.emit(UploadEvent::Progress {
            received: 8,
            total: None,
        });
        assert_eq!(
            upload.progress.get(),
            50,
            "an unknown total must not reset the bar"
        );
        assert_eq!(upload.status.get(), UploadStatus::Uploading);
    }

    #[tokio::test]
    async fn test_failure_reports_the_reason_and_keeps_the_file_empty() {
        let (services, service) = test_services();
        let mut store = HookStore::new();

        let (upload, _cleanups) = mount(&mut store, &services, UploadConstraints::new());
        let slot = slot_of(&service, &upload);

        slot.emit(UploadEvent::Failed(UploadError::TooLarge {
            limit: 10,
            actual: 99,
        }));

        match upload.status.get() {
            UploadStatus::Error(message) => {
                assert!(message.contains("99"), "got {message}");
                assert!(message.contains("10"), "got {message}");
            }
            other => panic!("expected Error, got {other:?}"),
        }
        assert!(upload.file.get().is_none());
    }

    #[tokio::test]
    async fn test_cancel_raises_the_flag_the_slot_reads() {
        let (services, service) = test_services();
        let mut store = HookStore::new();

        let (upload, _cleanups) = mount(&mut store, &services, UploadConstraints::new());
        let slot = slot_of(&service, &upload);
        assert!(!slot.is_cancelled());

        upload.cancel();
        assert!(slot.is_cancelled());

        // A slot resolved after the cancel sees it too — the endpoint checks
        // between chunks, so it may not have resolved the slot yet.
        assert!(slot_of(&service, &upload).is_cancelled());
    }

    #[tokio::test]
    async fn test_reset_clears_the_state_and_lowers_the_cancel_flag() {
        let (services, service) = test_services();
        let mut store = HookStore::new();

        let (upload, _cleanups) = mount(&mut store, &services, UploadConstraints::new());
        let slot = slot_of(&service, &upload);
        slot.emit(UploadEvent::Completed(a_file()));
        upload.cancel();

        upload.reset();

        assert_eq!(upload.status.get(), UploadStatus::Idle);
        assert_eq!(upload.progress.get(), 0);
        assert!(upload.file.get().is_none());
        assert!(!slot.is_cancelled());
        // The slot survives a reset: the URL is still usable.
        assert!(upload.url.get().is_some());
        assert_eq!(service.len(), 1);
    }

    #[tokio::test]
    async fn test_effect_cleanup_unregisters_the_slot() {
        let (services, service) = test_services();
        let mut store = HookStore::new();

        let (upload, cleanups) = mount(&mut store, &services, UploadConstraints::new());
        let url = upload.url.get().unwrap();
        let id = Uuid::parse_str(url.rsplit('/').next().unwrap()).unwrap();
        assert_eq!(service.len(), 1);

        // Unmount.
        for cleanup in cleanups {
            cleanup();
        }

        assert_eq!(service.len(), 0);
        assert!(service.slot(id).is_none());
    }

    #[tokio::test]
    async fn test_upload_is_cloneable_and_shares_its_state() {
        let (services, service) = test_services();
        let mut store = HookStore::new();

        let (upload, _cleanups) = mount(&mut store, &services, UploadConstraints::new());
        let captured = upload.clone();
        slot_of(&service, &upload).emit(UploadEvent::Completed(a_file()));

        assert_eq!(captured.status.get(), UploadStatus::Done);
        assert_eq!(captured.progress.get(), 100);
        assert!(format!("{captured:?}").contains("export.csv"));
    }

    #[test]
    #[should_panic(expected = "use_upload requires an UploadService")]
    fn test_missing_upload_service_panics() {
        let mut store = HookStore::new();
        let mut ctx = BuildContext::new(&mut store, None);
        let _ = use_upload(&mut ctx, UploadConstraints::new());
    }

    #[test]
    #[should_panic(expected = "use_upload requires an UploadService")]
    fn test_missing_upload_service_panics_for_the_sink_hook() {
        let mut store = HookStore::new();
        let mut ctx = BuildContext::new(&mut store, None);
        let _ = use_upload_to(&mut ctx, UploadConstraints::new(), |_| async { Ok(()) });
    }

    #[tokio::test]
    async fn test_use_upload_to_hands_the_bytes_to_the_sink_and_holds_nothing() {
        let (services, service) = test_services();
        let mut store = HookStore::new();
        let persisted = Arc::new(Mutex::new(Vec::<String>::new()));

        let (upload, _cleanups) = {
            let persisted = Arc::clone(&persisted);
            mount_with(&mut store, &services, move |ctx| {
                use_upload_to(ctx, UploadConstraints::new(), move |file| {
                    let persisted = Arc::clone(&persisted);
                    async move {
                        persisted.lock().unwrap().push(file.file_name.clone());
                        Ok(())
                    }
                })
            })
        };

        slot_of(&service, &upload).emit(UploadEvent::Completed(a_file()));

        let status = upload.status.clone();
        wait_until(move || status.get() == UploadStatus::Done).await;
        assert_eq!(upload.progress.get(), 100);
        assert!(
            upload.file.get().is_none(),
            "use_upload_to must not hold the bytes in view state"
        );
        assert_eq!(persisted.lock().unwrap().as_slice(), ["export.csv"]);
    }

    #[tokio::test]
    async fn test_use_upload_to_reports_a_sink_error_as_error() {
        let (services, service) = test_services();
        let mut store = HookStore::new();

        let (upload, _cleanups) = mount_with(&mut store, &services, |ctx| {
            use_upload_to(ctx, UploadConstraints::new(), |_file| async {
                Err(QueryError::new("disk is full"))
            })
        });

        slot_of(&service, &upload).emit(UploadEvent::Completed(a_file()));

        let status = upload.status.clone();
        wait_until(move || matches!(status.get(), UploadStatus::Error(_))).await;
        assert_eq!(
            upload.status.get(),
            UploadStatus::Error("disk is full".to_string())
        );
        assert!(upload.file.get().is_none());
        assert_ne!(upload.progress.get(), 100);
    }

    #[test]
    fn test_percent_received_caps_at_99() {
        assert_eq!(percent_received(0, 10), 0);
        assert_eq!(percent_received(5, 10), 50);
        assert_eq!(percent_received(10, 10), 99);
        // The multipart envelope makes received exceed the file's own length.
        assert_eq!(percent_received(20, 10), 99);
        assert_eq!(percent_received(u64::MAX, 3), 99);
    }
}
