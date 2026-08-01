use std::future::Future;
use std::sync::Arc;

use uuid::Uuid;

use crate::core::query_cache::{
    erase_fetcher, ErasedValue, QueryEntryState, QueryError, QueryOptions, QueryScope,
    QueryService, QuerySubscription,
};
use crate::core::services::AppContext;
use crate::hooks::use_ref::use_ref;
use crate::hooks::use_state::{use_state, State};
use crate::views::view::BuildContext;

/// Cache-mutating operations for one query key, handed back by [`use_query`] and
/// [`use_mutation`](crate::hooks::use_mutation::use_mutation).
///
/// Ported from Ivy-Framework's `QueryMutator<T>`. Cheap to clone (every operation
/// is an `Arc`'d closure), so it can be captured by event handlers.
pub struct QueryMutator<T> {
    mutate_fn: Arc<dyn Fn(Option<T>, bool) + Send + Sync>,
    revalidate_fn: Arc<dyn Fn() + Send + Sync>,
    invalidate_fn: Arc<dyn Fn() + Send + Sync>,
}

impl<T> QueryMutator<T> {
    /// A mutator whose operations do nothing — the idle result for `key: None`.
    pub fn empty() -> Self {
        QueryMutator {
            mutate_fn: Arc::new(|_, _| {}),
            revalidate_fn: Arc::new(|| {}),
            invalidate_fn: Arc::new(|| {}),
        }
    }

    /// Write `value` into the cache immediately (optimistic update). When
    /// `revalidate` is true the real value is fetched right after.
    pub fn mutate(&self, value: Option<T>, revalidate: bool) {
        (self.mutate_fn)(value, revalidate);
    }

    /// Optimistically set a value and refetch. The common mutation shape.
    pub fn set(&self, value: T) {
        (self.mutate_fn)(Some(value), true);
    }

    /// Refetch, keeping the current value visible while the fetch runs.
    pub fn revalidate(&self) {
        (self.revalidate_fn)();
    }

    /// Drop the cached value and refetch.
    pub fn invalidate(&self) {
        (self.invalidate_fn)();
    }
}

impl<T: Send + Sync + Clone + 'static> QueryMutator<T> {
    /// Build a mutator whose operations act on `key` in `service`.
    pub fn for_key(service: &Arc<QueryService>, key: &str) -> Self {
        let mutate_service = Arc::clone(service);
        let mutate_key = key.to_string();
        let revalidate_service = Arc::clone(service);
        let revalidate_key = key.to_string();
        let invalidate_service = Arc::clone(service);
        let invalidate_key = key.to_string();

        QueryMutator {
            mutate_fn: Arc::new(move |value: Option<T>, revalidate| {
                let erased = value.map(|v| Arc::new(v) as ErasedValue);
                mutate_service.mutate(&mutate_key, erased, revalidate);
            }),
            revalidate_fn: Arc::new(move || revalidate_service.revalidate(&revalidate_key)),
            invalidate_fn: Arc::new(move || invalidate_service.invalidate(&invalidate_key)),
        }
    }
}

impl<T> Clone for QueryMutator<T> {
    fn clone(&self) -> Self {
        QueryMutator {
            mutate_fn: Arc::clone(&self.mutate_fn),
            revalidate_fn: Arc::clone(&self.revalidate_fn),
            invalidate_fn: Arc::clone(&self.invalidate_fn),
        }
    }
}

impl<T> Default for QueryMutator<T> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> std::fmt::Debug for QueryMutator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("QueryMutator")
    }
}

/// What a view sees for one query. Ported from Ivy's `QueryResult<T>`.
#[derive(Debug, Clone)]
pub struct QueryResult<T> {
    /// The cached value, if any has been fetched (or optimistically written).
    pub value: Option<T>,
    /// A first fetch is in flight and there is nothing to show yet.
    pub loading: bool,
    /// A background revalidation is in flight; `value` is still displayable.
    pub validating: bool,
    /// `value` belongs to a previous key, kept visible via `keep_previous`.
    pub previous: bool,
    pub error: Option<QueryError>,
    pub mutator: QueryMutator<T>,
}

impl<T> QueryResult<T> {
    /// The idle result: no key, nothing loading, no-op mutator.
    pub fn idle() -> Self {
        QueryResult {
            value: None,
            loading: false,
            validating: false,
            previous: false,
            error: None,
            mutator: QueryMutator::empty(),
        }
    }

    /// True while there is neither a value nor an error to show.
    pub fn is_empty(&self) -> bool {
        self.value.is_none() && self.error.is_none()
    }
}

impl<T> Default for QueryResult<T> {
    fn default() -> Self {
        Self::idle()
    }
}

/// Per-view bookkeeping for a cache-backed query. Held in a `use_ref` so it
/// survives rebuilds; `Drop` on the `Arc<QuerySubscription>` unsubscribes when
/// the view's `HookStore` goes away.
#[derive(Clone)]
struct QuerySubscriptionSlot {
    subscriber_id: Uuid,
    key: Option<String>,
    subscription: Option<Arc<QuerySubscription>>,
}

/// Subscribe to a cached query and re-render as its state changes.
///
/// Ported from Ivy-Framework's `UseQuery.cs`. `key` is the cache key; pass `None`
/// for Ivy's conditional-fetch idle result (no subscription, no fetch). Callers
/// format their own keys (`format!("user:{id}")`) — Ivy's generic `TKey`
/// serialization and its `[CallerFilePath]` auto-key overload have no clean Rust
/// equivalent.
///
/// Requires a [`QueryService`] on the runtime's registry unless the scope is
/// [`QueryScope::View`], and an [`AppContext`] for [`QueryScope::App`].
pub fn use_query<T, F, Fut>(
    ctx: &mut BuildContext,
    key: Option<&str>,
    fetcher: F,
    options: QueryOptions,
) -> QueryResult<T>
where
    T: Send + Sync + Clone + 'static,
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, QueryError>> + Send + 'static,
{
    // Ivy throws when the scope changes between renders; the cache entry a
    // subscription belongs to would move out from under it.
    let scope_ref = use_ref(ctx, options.scope);
    let previous_scope = scope_ref.get();
    if previous_scope != options.scope {
        panic!(
            "use_query scope changed from {:?} to {:?} between renders. The scope decides which \
             cache entry a query belongs to and must be constant for a given call site.",
            previous_scope, options.scope
        );
    }

    if options.scope == QueryScope::View {
        return use_view_scoped_query(ctx, key, fetcher, options);
    }

    let scoped_key = key.map(|key| scoped_key(ctx, key, options.scope));
    let service = ctx
        .services()
        .get::<QueryService>()
        .expect("use_query requires a QueryService on the ServiceRegistry");

    let result_state = use_state(ctx, QueryResult::<T>::idle());
    let slot_ref = use_ref(
        ctx,
        QuerySubscriptionSlot {
            subscriber_id: Uuid::new_v4(),
            key: None,
            subscription: None,
        },
    );
    let slot = slot_ref.get();

    let Some(scoped_key) = scoped_key else {
        // Conditional fetch turned off: release any previous subscription and
        // report idle.
        if slot.subscription.is_some() {
            slot_ref.set(QuerySubscriptionSlot {
                subscriber_id: slot.subscriber_id,
                key: None,
                subscription: None,
            });
            drop(slot);
            result_state.set(QueryResult::idle());
        }
        return QueryResult::idle();
    };

    let key_changed = slot.key.as_deref() != Some(scoped_key.as_str());
    let mutator = QueryMutator::<T>::for_key(&service, &scoped_key);

    if key_changed {
        // Release the old subscription before subscribing to the new key, so the
        // old entry becomes evictable and its in-flight fetch is aborted.
        slot_ref.set(QuerySubscriptionSlot {
            subscriber_id: slot.subscriber_id,
            key: None,
            subscription: None,
        });
        drop(slot.subscription);

        let previous = result_state.get();
        let keep = options.keep_previous && previous.value.is_some();
        result_state.set(QueryResult {
            value: if keep { previous.value } else { None },
            loading: true,
            validating: false,
            previous: keep,
            error: None,
            mutator: mutator.clone(),
        });

        let subscription = subscribe(
            &service,
            &scoped_key,
            slot.subscriber_id,
            fetcher,
            &options,
            &result_state,
        );
        slot_ref.set(QuerySubscriptionSlot {
            subscriber_id: slot.subscriber_id,
            key: Some(scoped_key.clone()),
            subscription: Some(Arc::new(subscription)),
        });
    } else {
        // Same key: refresh the captured fetcher so revalidations see this render's
        // closure, and re-register the notifier against the current State.
        service.set_fetcher(&scoped_key, erase_fetcher(fetcher));
        service.set_subscriber(
            &scoped_key,
            slot.subscriber_id,
            notifier(&result_state, mutator.clone(), options.keep_previous),
        );
    }

    // Always hand back a mutator bound to the current key.
    let mut result = result_state.get();
    result.mutator = mutator;
    result
}

/// Subscribe `result_state` to `key`, registering the notifier first so no
/// notification fired during `subscribe` is missed.
fn subscribe<T, F, Fut>(
    service: &Arc<QueryService>,
    key: &str,
    subscriber_id: Uuid,
    fetcher: F,
    options: &QueryOptions,
    result_state: &State<QueryResult<T>>,
) -> QuerySubscription
where
    T: Send + Sync + Clone + 'static,
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, QueryError>> + Send + 'static,
{
    let mutator = QueryMutator::<T>::for_key(service, key);
    let subscription = service.subscribe(
        key,
        subscriber_id,
        erase_fetcher(fetcher),
        options.clone(),
        Vec::new(),
        None,
    );
    service.set_subscriber(
        key,
        subscriber_id,
        notifier(result_state, mutator, options.keep_previous),
    );
    // Catch up on whatever the state machine already decided during `subscribe`.
    service.notify_subscriber(key, subscriber_id);
    subscription
}

/// Map cache notifications onto `QueryResult` flags, exactly as Ivy's subscriber does.
fn notifier<T>(
    result_state: &State<QueryResult<T>>,
    mutator: QueryMutator<T>,
    keep_previous: bool,
) -> crate::core::query_cache::QuerySubscriberFn
where
    T: Send + Sync + Clone + 'static,
{
    let result_state = result_state.clone();
    Arc::new(move |value, state, error| {
        let typed = value.and_then(|v| v.downcast_ref::<T>().cloned());
        let previous_result = result_state.get();

        // While loading a new key with `keep_previous`, hold on to the old value.
        let (value, previous) = match &typed {
            Some(_) => (typed.clone(), false),
            None if keep_previous && previous_result.previous => {
                (previous_result.value.clone(), true)
            }
            None => (None, false),
        };

        result_state.set(QueryResult {
            value,
            loading: state == QueryEntryState::Fetching
                || (state == QueryEntryState::Empty && error.is_none()),
            validating: state == QueryEntryState::Revalidating,
            previous: previous && state != QueryEntryState::Fresh,
            error,
            mutator: mutator.clone(),
        });
    })
}

/// Prefix the key for connection-scoped queries. Server scope is global, so the
/// caller's key is used verbatim.
fn scoped_key(ctx: &BuildContext, key: &str, scope: QueryScope) -> String {
    match scope {
        QueryScope::App => {
            let app_context = ctx.services().get::<AppContext>().expect(
                "QueryScope::App requires an AppContext on the ServiceRegistry to scope the key by connection",
            );
            format!("{}:{}", app_context.connection_id, key)
        }
        QueryScope::Server | QueryScope::View => key.to_string(),
    }
}

/// `QueryScope::View` bypasses the cache entirely: the fetch belongs to this view,
/// so it runs on its own task and writes straight into local state.
///
/// A version counter guards against a stale completion overwriting a newer one
/// (`UseQuery.cs:263`) — the aborted-task equivalent of Ivy's cancellation token.
fn use_view_scoped_query<T, F, Fut>(
    ctx: &mut BuildContext,
    key: Option<&str>,
    fetcher: F,
    options: QueryOptions,
) -> QueryResult<T>
where
    T: Send + Sync + Clone + 'static,
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, QueryError>> + Send + 'static,
{
    let result_state = use_state(ctx, QueryResult::<T>::idle());
    let key_ref = use_ref(ctx, None::<String>);
    let version_ref = use_ref(ctx, 0u64);

    let Some(key) = key else {
        if key_ref.get().is_some() {
            // Bump the version so an in-flight fetch cannot land after we go idle.
            version_ref.update(|v| v + 1);
            key_ref.set(None);
            result_state.set(QueryResult::idle());
        }
        return QueryResult::idle();
    };

    if key_ref.get().as_deref() != Some(key) {
        key_ref.set(Some(key.to_string()));
        let version = version_ref.get() + 1;
        version_ref.set(version);

        let previous = result_state.get();
        let keep = options.keep_previous && previous.value.is_some();
        result_state.set(QueryResult {
            value: if keep { previous.value } else { None },
            loading: true,
            validating: false,
            previous: keep,
            error: None,
            mutator: QueryMutator::empty(),
        });

        let result_state = result_state.clone();
        let version_ref = version_ref.clone();
        tokio::spawn(async move {
            let outcome = fetcher().await;
            // A newer key was requested while this fetch was running — discard it.
            if version_ref.get() != version {
                return;
            }
            match outcome {
                Ok(value) => result_state.set(QueryResult {
                    value: Some(value),
                    loading: false,
                    validating: false,
                    previous: false,
                    error: None,
                    mutator: QueryMutator::empty(),
                }),
                Err(error) => {
                    let previous = result_state.get();
                    result_state.set(QueryResult {
                        value: previous.value,
                        loading: false,
                        validating: false,
                        previous: previous.previous,
                        error: Some(error),
                        mutator: QueryMutator::empty(),
                    });
                }
            }
        });
    }

    result_state.get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::services::ServiceRegistry;
    use crate::hooks::hook_store::HookStore;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use std::time::Duration;

    /// A registry with a `QueryService` and an `AppContext`, as a session has.
    fn test_services(connection_id: &str) -> Arc<ServiceRegistry> {
        let services = Arc::new(ServiceRegistry::new());
        services.register(Arc::new(QueryService::new()));
        services.register(Arc::new(AppContext::new(connection_id)));
        services
    }

    /// Let spawned fetch tasks run to completion.
    async fn settle() {
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
    }

    /// Build a view once against `store`, returning what `use_query` reported.
    fn build_once<T, F, Fut>(
        store: &mut HookStore,
        services: &Arc<ServiceRegistry>,
        key: Option<&str>,
        fetcher: F,
        options: QueryOptions,
    ) -> QueryResult<T>
    where
        T: Send + Sync + Clone + 'static,
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, QueryError>> + Send + 'static,
    {
        let mut ctx = BuildContext::with_services(store, None, uuid::Uuid::nil(), Arc::clone(services));
        use_query(&mut ctx, key, fetcher, options)
    }

    /// Boxed future alias so the test fetchers can share one return type.
    type FetchFuture<T> = std::pin::Pin<Box<dyn Future<Output = Result<T, QueryError>> + Send>>;

    /// A fetcher returning `value` and counting its calls.
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
    async fn test_use_query_loads_then_reports_value() {
        let services = test_services("conn-1");
        let mut store = HookStore::new();
        let calls = Arc::new(AtomicUsize::new(0));

        // First build: subscribes and starts the fetch.
        let first: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("user:1"),
            ok_fetcher("alice", calls.clone()),
            QueryOptions::default(),
        );
        assert!(first.loading, "the first build reports loading");
        assert!(first.value.is_none());

        settle().await;

        // Second build reads the resolved state out of the HookStore.
        let second: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("user:1"),
            ok_fetcher("alice", calls.clone()),
            QueryOptions::default(),
        );
        assert_eq!(second.value.as_deref(), Some("alice"));
        assert!(!second.loading);
        assert!(second.error.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_use_query_none_key_is_idle_and_never_fetches() {
        let services = test_services("conn-1");
        let mut store = HookStore::new();
        let calls = Arc::new(AtomicUsize::new(0));

        let result: QueryResult<String> = build_once(
            &mut store,
            &services,
            None,
            ok_fetcher("never", calls.clone()),
            QueryOptions::default(),
        );
        settle().await;

        assert!(result.value.is_none());
        assert!(!result.loading);
        assert!(result.is_empty());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            services.get::<QueryService>().unwrap().entry_count(),
            0,
            "a None key must not create a cache entry"
        );
    }

    #[tokio::test]
    async fn test_use_query_reports_error() {
        let services = test_services("conn-1");
        let mut store = HookStore::new();

        let fetcher =
            || Box::pin(async { Err::<String, _>(QueryError::new("boom")) }) as FetchFuture<String>;

        let _first: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("k"),
            fetcher,
            QueryOptions::default(),
        );
        settle().await;

        let second: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("k"),
            fetcher,
            QueryOptions::default(),
        );
        assert_eq!(second.error, Some(QueryError::new("boom")));
        assert!(second.value.is_none());
        assert!(
            !second.loading,
            "an errored empty entry is not still loading"
        );
    }

    #[tokio::test]
    async fn test_key_change_resubscribes_and_clears_value() {
        let services = test_services("conn-1");
        let mut store = HookStore::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let options = QueryOptions::default().expiration(Duration::from_secs(600));

        let _: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("page:1"),
            ok_fetcher("one", calls.clone()),
            options.clone(),
        );
        settle().await;
        let loaded: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("page:1"),
            ok_fetcher("one", calls.clone()),
            options.clone(),
        );
        assert_eq!(loaded.value.as_deref(), Some("one"));

        // Switch keys: value clears, loading goes true, and a new fetch starts.
        let switching: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("page:2"),
            ok_fetcher("two", calls.clone()),
            options.clone(),
        );
        assert!(switching.loading);
        assert!(
            switching.value.is_none(),
            "without keep_previous the old value clears"
        );
        settle().await;

        let switched: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("page:2"),
            ok_fetcher("two", calls.clone()),
            options,
        );
        assert_eq!(switched.value.as_deref(), Some("two"));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_keep_previous_holds_old_value_while_loading() {
        let services = test_services("conn-1");
        let mut store = HookStore::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let options = QueryOptions::default()
            .expiration(Duration::from_secs(600))
            .keep_previous(true);

        let _: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("page:1"),
            ok_fetcher("one", calls.clone()),
            options.clone(),
        );
        settle().await;
        let _: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("page:1"),
            ok_fetcher("one", calls.clone()),
            options.clone(),
        );

        let switching: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("page:2"),
            ok_fetcher("two", calls.clone()),
            options.clone(),
        );
        assert!(switching.loading);
        assert_eq!(
            switching.value.as_deref(),
            Some("one"),
            "keep_previous shows the old page while the new one loads"
        );
        assert!(switching.previous);

        settle().await;
        let switched: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("page:2"),
            ok_fetcher("two", calls.clone()),
            options,
        );
        assert_eq!(switched.value.as_deref(), Some("two"));
        assert!(
            !switched.previous,
            "previous clears once the new value lands"
        );
    }

    #[tokio::test]
    async fn test_mutator_set_updates_value_optimistically_and_revalidates() {
        let services = test_services("conn-1");
        let mut store = HookStore::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let options = QueryOptions::default().expiration(Duration::from_secs(600));

        let _: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("k"),
            ok_fetcher("server", calls.clone()),
            options.clone(),
        );
        settle().await;
        let loaded: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("k"),
            ok_fetcher("server", calls.clone()),
            options.clone(),
        );

        // Optimistic write without revalidation is visible immediately.
        loaded.mutator.mutate(Some("optimistic".to_string()), false);
        let after_mutate: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("k"),
            ok_fetcher("server", calls.clone()),
            options.clone(),
        );
        assert_eq!(after_mutate.value.as_deref(), Some("optimistic"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // `set` revalidates, so the server value wins.
        after_mutate.mutator.set("guess".to_string());
        settle().await;
        let after_set: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("k"),
            ok_fetcher("server", calls.clone()),
            options,
        );
        assert_eq!(after_set.value.as_deref(), Some("server"));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_mutator_revalidate_sets_validating_and_keeps_value() {
        let services = test_services("conn-1");
        let mut store = HookStore::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let options = QueryOptions::default().expiration(Duration::from_secs(600));

        let _: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("k"),
            ok_fetcher("v", calls.clone()),
            options.clone(),
        );
        settle().await;
        let loaded: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("k"),
            ok_fetcher("v", calls.clone()),
            options.clone(),
        );

        loaded.mutator.revalidate();
        let validating: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("k"),
            ok_fetcher("v", calls.clone()),
            options,
        );
        assert!(validating.validating, "revalidating is surfaced as a flag");
        assert_eq!(
            validating.value.as_deref(),
            Some("v"),
            "the stale value stays visible during revalidation"
        );
    }

    #[tokio::test]
    async fn test_app_scope_prefixes_key_with_connection_id() {
        let services = test_services("conn-abc");
        let mut store = HookStore::new();
        let calls = Arc::new(AtomicUsize::new(0));

        let _: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("cart"),
            ok_fetcher("v", calls.clone()),
            QueryOptions::default().scope(QueryScope::App),
        );
        settle().await;

        let service = services.get::<QueryService>().unwrap();
        assert_eq!(
            service.peek::<String>("conn-abc:cart").as_deref(),
            Some("v")
        );
        assert!(
            service.entry_state("cart").is_none(),
            "the unprefixed key must not be used"
        );
    }

    #[tokio::test]
    async fn test_two_views_share_one_server_scoped_entry() {
        let services = test_services("conn-1");
        let calls = Arc::new(AtomicUsize::new(0));
        let options = QueryOptions::default().expiration(Duration::from_secs(600));

        let mut store_a = HookStore::new();
        let mut store_b = HookStore::new();

        let _: QueryResult<String> = build_once(
            &mut store_a,
            &services,
            Some("shared"),
            ok_fetcher("v", calls.clone()),
            options.clone(),
        );
        settle().await;

        // A second view subscribing to the same key reuses the cached value.
        let b_first: QueryResult<String> = build_once(
            &mut store_b,
            &services,
            Some("shared"),
            ok_fetcher("v", calls.clone()),
            options,
        );
        assert_eq!(
            b_first.value.as_deref(),
            Some("v"),
            "a Fresh entry is delivered synchronously to the second view"
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "server-scoped queries share one fetch"
        );
        assert_eq!(services.get::<QueryService>().unwrap().entry_count(), 1);
    }

    #[tokio::test]
    async fn test_view_scope_bypasses_cache() {
        let services = test_services("conn-1");
        let mut store = HookStore::new();
        let calls = Arc::new(AtomicUsize::new(0));

        let first: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("local"),
            ok_fetcher("v", calls.clone()),
            QueryOptions::default().scope(QueryScope::View),
        );
        assert!(first.loading);
        settle().await;

        let second: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("local"),
            ok_fetcher("v", calls.clone()),
            QueryOptions::default().scope(QueryScope::View),
        );
        assert_eq!(second.value.as_deref(), Some("v"));
        assert_eq!(
            services.get::<QueryService>().unwrap().entry_count(),
            0,
            "View scope must not touch the shared cache"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_view_scope_discards_stale_completion() {
        let services = test_services("conn-1");
        let mut store = HookStore::new();
        let started = Arc::new(Mutex::new(Vec::<String>::new()));

        // A fetcher whose first key resolves slowly and second key resolves fast.
        let make_fetcher = |value: &str, delay_ms: u64, started: Arc<Mutex<Vec<String>>>| {
            let value = value.to_string();
            move || {
                let value = value.clone();
                let started = started.clone();
                Box::pin(async move {
                    started.lock().unwrap().push(value.clone());
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    Ok(value)
                }) as FetchFuture<String>
            }
        };

        let _: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("slow"),
            make_fetcher("slow-value", 500, started.clone()),
            QueryOptions::default().scope(QueryScope::View),
        );
        // Switch keys before the slow fetch completes.
        let _: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("fast"),
            make_fetcher("fast-value", 1, started.clone()),
            QueryOptions::default().scope(QueryScope::View),
        );

        tokio::time::sleep(Duration::from_millis(700)).await;
        settle().await;

        let final_result: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("fast"),
            make_fetcher("fast-value", 1, started.clone()),
            QueryOptions::default().scope(QueryScope::View),
        );
        assert_eq!(started.lock().unwrap().len(), 2, "both fetches started");
        assert_eq!(
            final_result.value.as_deref(),
            Some("fast-value"),
            "the stale slow completion must not overwrite the newer value"
        );
    }

    // Needs a runtime: the first build spawns a fetch before the second build panics.
    #[tokio::test]
    #[should_panic(expected = "use_query scope changed")]
    async fn test_scope_change_between_renders_panics() {
        let services = test_services("conn-1");
        let mut store = HookStore::new();
        let calls = Arc::new(AtomicUsize::new(0));

        let _: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("k"),
            ok_fetcher("v", calls.clone()),
            QueryOptions::default().scope(QueryScope::Server),
        );
        let _: QueryResult<String> = build_once(
            &mut store,
            &services,
            Some("k"),
            ok_fetcher("v", calls.clone()),
            QueryOptions::default().scope(QueryScope::App),
        );
    }
}
