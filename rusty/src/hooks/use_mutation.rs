use crate::core::query_cache::{QueryOptions, QueryScope, QueryService};
use crate::core::services::AppContext;
use crate::hooks::use_query::QueryMutator;
use crate::views::view::BuildContext;

/// Get a mutator for a query key without subscribing to it.
///
/// Ported from Ivy-Framework's `UseMutation` (`UseQuery.cs:447`). Use this in a
/// view that writes a query another view reads — a form that updates a list, say.
/// The mutation is visible to every subscriber of the key; this view is not
/// re-rendered by it, since it never subscribed.
///
/// Panics for [`QueryScope::View`] (as Ivy throws): a view-scoped query has no
/// cache entry to mutate.
///
/// Consumes no hook slot — the returned mutator is derived from the key and the
/// service, with no per-view state.
pub fn use_mutation<T>(ctx: &BuildContext, key: &str, options: QueryOptions) -> QueryMutator<T>
where
    T: Send + Sync + Clone + 'static,
{
    assert!(
        options.scope != QueryScope::View,
        "use_mutation cannot be used with QueryScope::View — a view-scoped query is not in the \
         shared cache, so there is nothing to mutate."
    );

    let service = ctx
        .services()
        .get::<QueryService>()
        .expect("use_mutation requires a QueryService on the ServiceRegistry");

    let scoped_key = match options.scope {
        QueryScope::App => {
            let app_context = ctx.services().get::<AppContext>().expect(
                "QueryScope::App requires an AppContext on the ServiceRegistry to scope the key by connection",
            );
            format!("{}:{}", app_context.connection_id, key)
        }
        QueryScope::Server | QueryScope::View => key.to_string(),
    };

    QueryMutator::for_key(&service, &scoped_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::query_cache::{QueryEntryState, QueryError};
    use crate::core::services::ServiceRegistry;
    use crate::hooks::hook_store::HookStore;
    use crate::hooks::use_query::{use_query, QueryResult};
    use std::future::Future;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    type FetchFuture<T> = std::pin::Pin<Box<dyn Future<Output = Result<T, QueryError>> + Send>>;

    fn test_services(connection_id: &str) -> Arc<ServiceRegistry> {
        let services = Arc::new(ServiceRegistry::new());
        services.register(Arc::new(QueryService::new()));
        services.register(Arc::new(AppContext::new(connection_id)));
        services
    }

    async fn settle() {
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
    }

    fn ok_fetcher(value: &str, calls: Arc<AtomicUsize>) -> impl Fn() -> FetchFuture<String> {
        let value = value.to_string();
        move || {
            let value = value.clone();
            let calls = calls.clone();
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(value)
            }) as FetchFuture<String>
        }
    }

    #[tokio::test]
    async fn test_use_mutation_mutates_a_query_another_view_reads() {
        let services = test_services("conn-1");
        let calls = Arc::new(AtomicUsize::new(0));
        let options = QueryOptions::default().expiration(Duration::from_secs(600));

        // A reader view subscribes to the key.
        let mut reader_store = HookStore::new();
        {
            let mut ctx =
                BuildContext::new(&mut reader_store, None).with_services(Arc::clone(&services));
            let _: QueryResult<String> = use_query(
                &mut ctx,
                Some("todos"),
                ok_fetcher("from-server", calls.clone()),
                options.clone(),
            );
        }
        settle().await;

        // A writer view mutates it without subscribing.
        let mut writer_store = HookStore::new();
        let mutator: QueryMutator<String> = {
            let ctx =
                BuildContext::new(&mut writer_store, None).with_services(Arc::clone(&services));
            use_mutation(&ctx, "todos", options.clone())
        };
        mutator.mutate(Some("written".to_string()), false);

        // The reader sees the new value on its next build.
        let reader_result: QueryResult<String> = {
            let mut ctx =
                BuildContext::new(&mut reader_store, None).with_services(Arc::clone(&services));
            use_query(
                &mut ctx,
                Some("todos"),
                ok_fetcher("from-server", calls.clone()),
                options,
            )
        };
        assert_eq!(reader_result.value.as_deref(), Some("written"));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "mutate without revalidate must not refetch"
        );
    }

    #[tokio::test]
    async fn test_use_mutation_scopes_key_by_connection_for_app_scope() {
        let services = test_services("conn-xyz");
        let calls = Arc::new(AtomicUsize::new(0));
        let options = QueryOptions::default().scope(QueryScope::App);

        let mut reader_store = HookStore::new();
        {
            let mut ctx =
                BuildContext::new(&mut reader_store, None).with_services(Arc::clone(&services));
            let _: QueryResult<String> = use_query(
                &mut ctx,
                Some("cart"),
                ok_fetcher("v", calls.clone()),
                options.clone(),
            );
        }
        settle().await;

        let mut writer_store = HookStore::new();
        let mutator: QueryMutator<String> = {
            let ctx =
                BuildContext::new(&mut writer_store, None).with_services(Arc::clone(&services));
            use_mutation(&ctx, "cart", options)
        };
        mutator.invalidate();

        let service = services.get::<QueryService>().unwrap();
        assert_eq!(
            service.peek::<String>("conn-xyz:cart"),
            None,
            "invalidate must have cleared the connection-scoped entry"
        );
        assert_eq!(
            service.entry_state("conn-xyz:cart"),
            Some(QueryEntryState::Fetching),
            "the reader is still subscribed, so invalidate refetches"
        );
    }

    #[test]
    #[should_panic(expected = "use_mutation cannot be used with QueryScope::View")]
    fn test_use_mutation_panics_for_view_scope() {
        let services = test_services("conn-1");
        let mut store = HookStore::new();
        let ctx = BuildContext::new(&mut store, None).with_services(services);
        let _: QueryMutator<String> =
            use_mutation(&ctx, "k", QueryOptions::default().scope(QueryScope::View));
    }
}
