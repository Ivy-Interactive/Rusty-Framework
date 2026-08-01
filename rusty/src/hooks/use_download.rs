use std::future::Future;
use std::sync::Arc;

use crate::core::query_cache::QueryError;
use crate::hooks::use_effect::use_effect;
use crate::hooks::use_state::{use_state, State};
use crate::server::download::{download_factory, DownloadService};
use crate::views::view::BuildContext;

/// Register a download and get back the URL to serve it from.
///
/// Ported from Ivy-Framework's `UseDownload.cs`. The URL state starts as `None`
/// and is filled in once the mount effect registers the download, so a view
/// typically renders the link only when the state is `Some`.
///
/// `factory` runs when the client requests the URL, not at registration — so
/// offering a download costs nothing until it is taken. The registration is
/// released in the effect cleanup, which the runtime runs on unmount.
///
/// Ivy's `Stream` overload is out of scope: Rusty has no streaming-body plumbing,
/// so the bytes are produced whole.
pub fn use_download<F, Fut>(
    ctx: &mut BuildContext,
    factory: F,
    mime_type: impl Into<String>,
    file_name: impl Into<String>,
) -> State<Option<String>>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Vec<u8>, QueryError>> + Send + 'static,
{
    let url_state = use_state(ctx, None::<String>);
    let service = ctx
        .services()
        .get::<DownloadService>()
        .expect("use_download requires a DownloadService on the ServiceRegistry");

    let mime_type = mime_type.into();
    let file_name = file_name.into();
    let effect_state = url_state.clone();

    use_effect(ctx, move || {
        let (handle, url) = service.add_download(download_factory(factory), mime_type, file_name);
        effect_state.set(Some(url));

        // Dropping the handle unregisters the download.
        Some(Box::new(move || {
            drop(handle);
        }) as Box<dyn FnOnce() + Send + Sync>)
    });

    url_state
}

/// Register a download whose bytes are already in hand.
///
/// The common case where a view has the content and only needs a URL for it.
pub fn use_download_bytes(
    ctx: &mut BuildContext,
    bytes: Vec<u8>,
    mime_type: impl Into<String>,
    file_name: impl Into<String>,
) -> State<Option<String>> {
    let bytes = Arc::new(bytes);
    use_download(
        ctx,
        move || {
            let bytes = Arc::clone(&bytes);
            async move { Ok(bytes.as_ref().clone()) }
        },
        mime_type,
        file_name,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::services::ServiceRegistry;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::EffectCleanup;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use uuid::Uuid;

    fn test_services() -> (Arc<ServiceRegistry>, Arc<DownloadService>) {
        let download_service = Arc::new(DownloadService::new("conn-1"));
        let services = Arc::new(ServiceRegistry::new());
        services.register(Arc::clone(&download_service));
        (services, download_service)
    }

    /// Build once, run the effects, and return the URL state plus any cleanups.
    fn build_and_run_effects(
        store: &mut HookStore,
        services: &Arc<ServiceRegistry>,
        bytes: Vec<u8>,
    ) -> (State<Option<String>>, Vec<EffectCleanup>) {
        let (state, effects) = {
            let mut ctx = BuildContext::new(store, None).using_services(Arc::clone(services));
            let state = use_download_bytes(&mut ctx, bytes, "text/csv", "export.csv");
            (state, ctx.drain_effects())
        };

        let mut cleanups = Vec::new();
        for effect in effects {
            if let Some(cleanup) = (effect.callback)() {
                cleanups.push(cleanup);
            }
        }
        (state, cleanups)
    }

    #[tokio::test]
    async fn test_use_download_sets_the_url_after_the_effect_runs() {
        let (services, download_service) = test_services();
        let mut store = HookStore::new();

        let (state, _cleanups) = build_and_run_effects(&mut store, &services, b"a,b,c".to_vec());

        let url = state.get().expect("the effect should have set the url");
        assert!(
            url.starts_with("/rusty/download/conn-1/"),
            "unexpected url: {url}"
        );
        assert_eq!(download_service.len(), 1);
    }

    #[tokio::test]
    async fn test_registered_download_serves_the_bytes() {
        let (services, download_service) = test_services();
        let mut store = HookStore::new();

        let (state, _cleanups) =
            build_and_run_effects(&mut store, &services, b"id,name\n1,alice".to_vec());

        let url = state.get().unwrap();
        let download_id = Uuid::parse_str(url.rsplit('/').next().unwrap()).unwrap();
        let response = download_service.take_bytes(download_id).await.unwrap();

        assert_eq!(response.bytes, b"id,name\n1,alice".to_vec());
        assert_eq!(response.mime_type, "text/csv");
        assert_eq!(response.file_name, "export.csv");
    }

    #[tokio::test]
    async fn test_effect_cleanup_unregisters_the_download() {
        let (services, download_service) = test_services();
        let mut store = HookStore::new();

        let (state, cleanups) = build_and_run_effects(&mut store, &services, b"x".to_vec());
        let url = state.get().unwrap();
        let download_id = Uuid::parse_str(url.rsplit('/').next().unwrap()).unwrap();
        assert_eq!(download_service.len(), 1);

        // Unmount.
        for cleanup in cleanups {
            cleanup();
        }
        assert_eq!(download_service.len(), 0);
        assert!(download_service.take_bytes(download_id).await.is_none());
    }

    #[tokio::test]
    async fn test_factory_is_not_called_until_the_download_is_taken() {
        let (services, download_service) = test_services();
        let mut store = HookStore::new();
        let calls = Arc::new(AtomicUsize::new(0));

        let (state, _cleanups) = {
            let calls = calls.clone();
            let (state, effects) = {
                let mut ctx =
                    BuildContext::new(&mut store, None).using_services(Arc::clone(&services));
                let state = use_download(
                    &mut ctx,
                    move || {
                        let calls = calls.clone();
                        async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            Ok(b"generated".to_vec())
                        }
                    },
                    "application/json",
                    "data.json",
                );
                (state, ctx.drain_effects())
            };
            let mut cleanups = Vec::new();
            for effect in effects {
                if let Some(cleanup) = (effect.callback)() {
                    cleanups.push(cleanup);
                }
            }
            (state, cleanups)
        };

        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "registering a download must not generate its bytes"
        );

        let url = state.get().unwrap();
        let download_id = Uuid::parse_str(url.rsplit('/').next().unwrap()).unwrap();
        let response = download_service.take_bytes(download_id).await.unwrap();
        assert_eq!(response.bytes, b"generated".to_vec());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    #[should_panic(expected = "use_download requires a DownloadService")]
    fn test_missing_download_service_panics() {
        let mut store = HookStore::new();
        let mut ctx = BuildContext::new(&mut store, None);
        let _ = use_download_bytes(&mut ctx, vec![], "text/plain", "x.txt");
    }
}
