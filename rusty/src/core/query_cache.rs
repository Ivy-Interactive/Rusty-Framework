use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio::time::Instant;
use uuid::Uuid;

/// Where a query's cache entry lives.
///
/// Ported from Ivy-Framework's `QueryScope`. Ivy's `Device` and `User` scopes are
/// dropped: Rusty has neither a machine id nor an auth service to key them on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QueryScope {
    /// Shared by every connection on the server.
    #[default]
    Server,
    /// Not cached at all — the fetch is owned by the view that requested it.
    View,
    /// Scoped to one connection by prefixing the key with the connection id.
    App,
}

/// Per-query behaviour, mirroring Ivy's `QueryOptions`.
#[derive(Debug, Clone)]
pub struct QueryOptions {
    /// How long a fetched value stays `Fresh`. `None` means it never goes stale on its own.
    pub expiration: Option<Duration>,
    pub scope: QueryScope,
    /// Revalidate in the background on subscribe (the SWR pattern). Default `true`.
    ///
    /// When `false` with an initial value, the entry is populated without fetching.
    pub revalidate_on_mount: bool,
    /// Keep showing the previous value while a new key is loading (pagination).
    pub keep_previous: bool,
    /// Revalidate on this interval while the entry has subscribers.
    pub refresh_interval: Option<Duration>,
}

impl Default for QueryOptions {
    fn default() -> Self {
        QueryOptions {
            expiration: None,
            scope: QueryScope::Server,
            revalidate_on_mount: true,
            keep_previous: false,
            refresh_interval: None,
        }
    }
}

impl QueryOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scope(mut self, scope: QueryScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn expiration(mut self, expiration: Duration) -> Self {
        self.expiration = Some(expiration);
        self
    }

    pub fn revalidate_on_mount(mut self, revalidate: bool) -> Self {
        self.revalidate_on_mount = revalidate;
        self
    }

    pub fn keep_previous(mut self, keep: bool) -> Self {
        self.keep_previous = keep;
        self
    }

    pub fn refresh_interval(mut self, interval: Duration) -> Self {
        self.refresh_interval = Some(interval);
        self
    }
}

impl From<QueryScope> for QueryOptions {
    fn from(scope: QueryScope) -> Self {
        QueryOptions::default().scope(scope)
    }
}

/// Service-wide cache tuning, mirroring Ivy's `QueryServiceOptions`.
#[derive(Debug, Clone)]
pub struct QueryServiceOptions {
    /// How often to scan for evictable entries.
    pub eviction_interval: Duration,
    /// How long an entry with no subscribers survives.
    pub orphaned_entry_ttl: Duration,
    /// Entry count above which orphans are evicted LRU-first. `None` = unlimited.
    pub max_entries: Option<usize>,
    /// How often to check for entries due for a `refresh_interval` revalidation.
    pub refresh_tick_interval: Duration,
}

impl Default for QueryServiceOptions {
    fn default() -> Self {
        QueryServiceOptions {
            eviction_interval: Duration::from_secs(60),
            orphaned_entry_ttl: Duration::from_secs(60 * 60),
            max_entries: Some(10_000),
            refresh_tick_interval: Duration::from_secs(1),
        }
    }
}

/// Lifecycle state of a cache entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryEntryState {
    /// No value and no fetch in progress.
    Empty,
    /// First fetch in progress, nothing cached to show.
    Fetching,
    /// Value present and within its TTL.
    Fresh,
    /// Value present but past its TTL.
    Stale,
    /// Stale value present while a background refresh runs.
    Revalidating,
}

/// A fetch failure. `Send + Sync` and cloneable so it can be handed to every subscriber.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryError {
    pub message: String,
}

impl QueryError {
    pub fn new(message: impl Into<String>) -> Self {
        QueryError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for QueryError {}

impl From<String> for QueryError {
    fn from(message: String) -> Self {
        QueryError::new(message)
    }
}

impl From<&str> for QueryError {
    fn from(message: &str) -> Self {
        QueryError::new(message)
    }
}

/// A cached value, type-erased so one cache can hold every query's value type.
/// Subscribers get a cheap `Arc` clone and recover `T` with `downcast_ref`.
pub type ErasedValue = Arc<dyn Any + Send + Sync>;

/// A type-erased fetcher. Called with no arguments — the key is captured by the closure.
pub type ErasedFetcher = Arc<
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<ErasedValue, QueryError>> + Send>> + Send + Sync,
>;

/// A subscriber notification callback: `(value, state, error)`.
///
/// **Must not call back into `QueryService`** — notification runs while the cache
/// lock is held, and the lock is not reentrant.
pub type QuerySubscriberFn =
    Arc<dyn Fn(Option<ErasedValue>, QueryEntryState, Option<QueryError>) + Send + Sync>;

/// Wrap a typed async fetcher into an [`ErasedFetcher`].
pub fn erase_fetcher<T, F, Fut>(fetcher: F) -> ErasedFetcher
where
    T: Send + Sync + 'static,
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, QueryError>> + Send + 'static,
{
    Arc::new(move || {
        let fut = fetcher();
        Box::pin(async move { fut.await.map(|value| Arc::new(value) as ErasedValue) })
    })
}

/// One entry in the cache. Mirrors Ivy's `QueryEntry`.
///
/// Ivy's `Key` and `CreatedAt` fields are dropped: the cache's `HashMap` key is
/// already the query key, and nothing reads a creation timestamp — eviction works
/// off `last_accessed_at` and staleness off `expires_at`.
struct QueryEntry {
    value: Option<ErasedValue>,
    error: Option<QueryError>,
    state: QueryEntryState,
    fetcher: Option<ErasedFetcher>,
    options: QueryOptions,
    last_fetched_at: Option<Instant>,
    last_accessed_at: Instant,
    expires_at: Option<Instant>,
    tags: Vec<String>,
    subscribers: HashMap<Uuid, QuerySubscriberFn>,
    /// The in-flight fetch. Cancellation is `abort()` — Rust's equivalent of
    /// Ivy's `CancellationTokenSource`.
    fetch_handle: Option<JoinHandle<()>>,
}

impl QueryEntry {
    fn is_orphaned(&self) -> bool {
        self.subscribers.is_empty()
    }

    fn notify(&self) {
        for subscriber in self.subscribers.values() {
            subscriber(self.value.clone(), self.state, self.error.clone());
        }
    }

    fn abort_fetch(&mut self) {
        if let Some(handle) = self.fetch_handle.take() {
            handle.abort();
        }
    }
}

/// An SWR-style query cache, ported from Ivy-Framework's `QueryService`.
///
/// **Invariant: the cache lock is never held across an `.await`.** Every public
/// method is synchronous; fetches run on spawned tasks that re-acquire the lock
/// to store their result.
pub struct QueryService {
    cache: Mutex<HashMap<String, QueryEntry>>,
    options: QueryServiceOptions,
}

/// Unsubscribes on drop, replacing Ivy's `IDisposable` subscription handle.
pub struct QuerySubscription {
    service: Weak<QueryService>,
    key: String,
    subscriber_id: Uuid,
}

impl Drop for QuerySubscription {
    fn drop(&mut self) {
        if let Some(service) = self.service.upgrade() {
            service.unsubscribe(&self.key, self.subscriber_id);
        }
    }
}

impl std::fmt::Debug for QuerySubscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuerySubscription")
            .field("key", &self.key)
            .field("subscriber_id", &self.subscriber_id)
            .finish()
    }
}

impl QueryService {
    pub fn new() -> Self {
        Self::with_options(QueryServiceOptions::default())
    }

    pub fn with_options(options: QueryServiceOptions) -> Self {
        QueryService {
            cache: Mutex::new(HashMap::new()),
            options,
        }
    }

    /// Spawn the eviction and refresh tickers. The returned handles abort the
    /// tickers when dropped-and-aborted, standing in for Ivy's `Timer` disposal.
    pub fn start_background_tasks(self: &Arc<Self>) -> Vec<JoinHandle<()>> {
        let eviction_service = Arc::clone(self);
        let eviction_interval = self.options.eviction_interval;
        let eviction = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(eviction_interval);
            ticker.tick().await; // the first tick is immediate
            loop {
                ticker.tick().await;
                eviction_service.evict_expired_entries();
            }
        });

        let refresh_service = Arc::clone(self);
        let refresh_interval = self.options.refresh_tick_interval;
        let refresh = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(refresh_interval);
            ticker.tick().await;
            loop {
                ticker.tick().await;
                refresh_service.refresh_due_entries();
            }
        });

        vec![eviction, refresh]
    }

    /// Number of entries currently cached.
    pub fn entry_count(&self) -> usize {
        self.cache.lock().unwrap().len()
    }

    /// Current state of an entry, or `None` if it is not cached.
    pub fn entry_state(&self, key: &str) -> Option<QueryEntryState> {
        self.cache.lock().unwrap().get(key).map(|e| e.state)
    }

    /// Current value of an entry, downcast to `T`.
    pub fn peek<T: Send + Sync + Clone + 'static>(&self, key: &str) -> Option<T> {
        let cache = self.cache.lock().unwrap();
        cache.get(key)?.value.as_ref()?.downcast_ref::<T>().cloned()
    }

    /// Subscribe to a query entry, creating it if needed, and drive the SWR state
    /// machine. The returned handle unsubscribes on drop.
    ///
    /// Ported from `QueryService.Subscribe` plus `HandleSubscriptionAsync`; the
    /// whole state machine runs under the cache lock so that concurrent
    /// subscribers to one key start exactly one fetch.
    pub fn subscribe(
        self: &Arc<Self>,
        key: &str,
        subscriber_id: Uuid,
        fetcher: ErasedFetcher,
        options: QueryOptions,
        tags: Vec<String>,
        initial_value: Option<ErasedValue>,
    ) -> QuerySubscription {
        let now = Instant::now();
        let mut spawn_fetch = None;

        {
            let mut cache = self.cache.lock().unwrap();
            let entry = cache.entry(key.to_string()).or_insert_with(|| QueryEntry {
                value: None,
                error: None,
                state: QueryEntryState::Empty,
                fetcher: None,
                options: options.clone(),
                last_fetched_at: None,
                last_accessed_at: now,
                expires_at: None,
                tags,
                subscribers: HashMap::new(),
                fetch_handle: None,
            });

            // The fetcher closure is rebuilt every render, so always take the latest.
            entry.fetcher = Some(fetcher);
            entry.options = options.clone();
            entry.last_accessed_at = now;

            // Populate from an initial value when the caller opted out of fetching.
            if !options.revalidate_on_mount && initial_value.is_some() && entry.value.is_none() {
                entry.value = initial_value;
                entry.state = QueryEntryState::Fresh;
                entry.last_fetched_at = Some(now);
                entry.expires_at = options.expiration.map(|exp| now + exp);
            }

            // The callback itself arrives via `set_subscriber`; registering a no-op
            // placeholder now keeps the entry from looking orphaned in between.
            entry
                .subscribers
                .entry(subscriber_id)
                .or_insert_with(|| Arc::new(|_, _, _| {}));

            // A `Fresh` entry past its TTL is really `Stale`.
            if entry.state == QueryEntryState::Fresh {
                if let Some(expires_at) = entry.expires_at {
                    if now > expires_at {
                        entry.state = QueryEntryState::Stale;
                    }
                }
            }

            match entry.state {
                QueryEntryState::Empty => {
                    spawn_fetch = Self::begin_fetch(entry, false);
                }
                QueryEntryState::Fresh => {
                    entry.notify();
                    // SWR: revalidate in the background unless the caller opted out
                    // or is using TTL-based staleness instead.
                    if options.revalidate_on_mount && entry.options.expiration.is_none() {
                        spawn_fetch = Self::begin_fetch(entry, true);
                    }
                }
                QueryEntryState::Stale => {
                    entry.notify();
                    spawn_fetch = Self::begin_fetch(entry, true);
                }
                QueryEntryState::Fetching | QueryEntryState::Revalidating => {
                    // A fetch is already running; this subscriber will be notified
                    // when it completes.
                    entry.notify();
                }
            }
        }

        if let Some(fetcher) = spawn_fetch {
            self.spawn_fetch(key.to_string(), fetcher);
        }

        QuerySubscription {
            service: Arc::downgrade(self),
            key: key.to_string(),
            subscriber_id,
        }
    }

    /// Replace an entry's fetcher without touching its state.
    ///
    /// A hook rebuilds its fetcher closure every render, capturing whatever the
    /// view read this time; without this the entry would keep revalidating through
    /// the closure captured on the first render.
    pub fn set_fetcher(&self, key: &str, fetcher: ErasedFetcher) {
        let mut cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get_mut(key) {
            entry.fetcher = Some(fetcher);
        }
    }

    /// Register (or replace) a subscriber callback for an already-subscribed id.
    ///
    /// Split out from `subscribe` because a hook builds its callback around the
    /// same `State` it hands to the caller; registering it separately keeps
    /// `subscribe`'s signature free of generics.
    pub fn set_subscriber(&self, key: &str, subscriber_id: Uuid, subscriber: QuerySubscriberFn) {
        let mut cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get_mut(key) {
            entry.subscribers.insert(subscriber_id, subscriber);
        }
    }

    /// Notify a single subscriber with the entry's current state, so a freshly
    /// registered callback sees what it missed.
    pub fn notify_subscriber(&self, key: &str, subscriber_id: Uuid) {
        let cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get(key) {
            if let Some(subscriber) = entry.subscribers.get(&subscriber_id) {
                subscriber(entry.value.clone(), entry.state, entry.error.clone());
            }
        }
    }

    /// Optimistic update, optionally followed by a revalidation.
    /// Ported from `QueryService.Mutate`.
    pub fn mutate(self: &Arc<Self>, key: &str, value: Option<ErasedValue>, revalidate: bool) {
        let now = Instant::now();
        let mut spawn_fetch = None;

        {
            let mut cache = self.cache.lock().unwrap();
            let Some(entry) = cache.get_mut(key) else {
                return;
            };

            entry.value = value;
            entry.error = None;
            entry.last_accessed_at = now;

            if revalidate && entry.fetcher.is_some() {
                entry.abort_fetch();
                entry.state = QueryEntryState::Revalidating;
                entry.notify();
                spawn_fetch = entry.fetcher.clone();
            } else {
                entry.state = QueryEntryState::Fresh;
                entry.last_fetched_at = Some(now);
                entry.expires_at = entry.options.expiration.map(|exp| now + exp);
                entry.notify();
            }
        }

        if let Some(fetcher) = spawn_fetch {
            self.spawn_fetch(key.to_string(), fetcher);
        }
    }

    /// Force a revalidation of one entry, keeping any cached value visible.
    pub fn revalidate(self: &Arc<Self>, key: &str) {
        let spawn_fetch;

        {
            let mut cache = self.cache.lock().unwrap();
            let Some(entry) = cache.get_mut(key) else {
                return;
            };
            if entry.fetcher.is_none() {
                return;
            }

            entry.abort_fetch();
            entry.error = None;
            entry.state = if entry.value.is_some() {
                QueryEntryState::Revalidating
            } else {
                QueryEntryState::Fetching
            };
            entry.notify();
            spawn_fetch = entry.fetcher.clone();
        }

        if let Some(fetcher) = spawn_fetch {
            self.spawn_fetch(key.to_string(), fetcher);
        }
    }

    /// Drop an entry's cached value and refetch if anyone is still subscribed.
    pub fn invalidate(self: &Arc<Self>, key: &str) {
        let mut spawn_fetch = None;

        {
            let mut cache = self.cache.lock().unwrap();
            let Some(entry) = cache.get_mut(key) else {
                return;
            };

            entry.abort_fetch();
            entry.value = None;
            entry.error = None;

            if !entry.is_orphaned() && entry.fetcher.is_some() {
                entry.state = QueryEntryState::Fetching;
                entry.notify();
                spawn_fetch = entry.fetcher.clone();
            } else {
                // No subscribers — mark empty and let eviction collect it.
                entry.state = QueryEntryState::Empty;
                entry.notify();
            }
        }

        if let Some(fetcher) = spawn_fetch {
            self.spawn_fetch(key.to_string(), fetcher);
        }
    }

    /// Invalidate every entry carrying `tag`.
    pub fn invalidate_by_tag(self: &Arc<Self>, tag: &str) {
        for key in self.keys_with_tag(tag) {
            self.invalidate(&key);
        }
    }

    /// Revalidate every entry carrying `tag`.
    pub fn revalidate_by_tag(self: &Arc<Self>, tag: &str) {
        for key in self.keys_with_tag(tag) {
            self.revalidate(&key);
        }
    }

    /// Invalidate every entry whose key satisfies `predicate`.
    pub fn invalidate_where(self: &Arc<Self>, predicate: impl Fn(&str) -> bool) {
        for key in self.keys_where(predicate) {
            self.invalidate(&key);
        }
    }

    /// Revalidate every entry whose key satisfies `predicate`.
    pub fn revalidate_where(self: &Arc<Self>, predicate: impl Fn(&str) -> bool) {
        for key in self.keys_where(predicate) {
            self.revalidate(&key);
        }
    }

    /// Invalidate every entry.
    pub fn clear(self: &Arc<Self>) {
        let keys: Vec<String> = self.cache.lock().unwrap().keys().cloned().collect();
        for key in keys {
            self.invalidate(&key);
        }
    }

    /// Remove a subscriber and cancel the in-flight fetch once nobody is left.
    pub fn unsubscribe(&self, key: &str, subscriber_id: Uuid) {
        let mut cache = self.cache.lock().unwrap();
        let Some(entry) = cache.get_mut(key) else {
            return;
        };

        entry.subscribers.remove(&subscriber_id);

        if entry.is_orphaned() && entry.fetch_handle.is_some() {
            entry.abort_fetch();
            // Ivy resets the state in its cancellation handler; `abort()` has no
            // such hook, so roll the state back here instead.
            if matches!(
                entry.state,
                QueryEntryState::Fetching | QueryEntryState::Revalidating
            ) {
                entry.state = if entry.value.is_some() {
                    QueryEntryState::Stale
                } else {
                    QueryEntryState::Empty
                };
            }
        }
    }

    /// Evict entries per Ivy's rules: expired-and-orphaned, orphaned past TTL,
    /// `Empty`-and-orphaned, then LRU over orphans above `max_entries`.
    pub fn evict_expired_entries(&self) {
        let now = Instant::now();
        let mut cache = self.cache.lock().unwrap();

        let mut to_remove: Vec<String> = Vec::new();
        for (key, entry) in cache.iter() {
            let orphaned = entry.is_orphaned();
            let expired = entry.expires_at.is_some_and(|expires| now > expires);
            let stale_orphan = now > entry.last_accessed_at + self.options.orphaned_entry_ttl;
            let empty = entry.state == QueryEntryState::Empty;

            if orphaned && (expired || stale_orphan || empty) {
                to_remove.push(key.clone());
            }
        }

        for key in to_remove {
            if let Some(mut entry) = cache.remove(&key) {
                entry.abort_fetch();
            }
        }

        // LRU eviction over orphans once above the entry cap.
        if let Some(max_entries) = self.options.max_entries {
            if cache.len() > max_entries {
                let over = cache.len() - max_entries;
                let mut candidates: Vec<(String, Instant)> = cache
                    .iter()
                    .filter(|(_, entry)| entry.is_orphaned())
                    .map(|(key, entry)| (key.clone(), entry.last_accessed_at))
                    .collect();
                candidates.sort_by_key(|(_, last_accessed)| *last_accessed);

                for (key, _) in candidates.into_iter().take(over) {
                    if let Some(mut entry) = cache.remove(&key) {
                        entry.abort_fetch();
                    }
                }
            }
        }
    }

    /// Revalidate subscribed entries whose `refresh_interval` has elapsed.
    pub fn refresh_due_entries(self: &Arc<Self>) {
        let now = Instant::now();
        let due: Vec<String> = {
            let cache = self.cache.lock().unwrap();
            cache
                .iter()
                .filter(|(_, entry)| {
                    if entry.is_orphaned() {
                        return false;
                    }
                    let Some(interval) = entry.options.refresh_interval else {
                        return false;
                    };
                    if matches!(
                        entry.state,
                        QueryEntryState::Fetching | QueryEntryState::Revalidating
                    ) {
                        return false;
                    }
                    entry
                        .last_fetched_at
                        .is_some_and(|fetched| now >= fetched + interval)
                })
                .map(|(key, _)| key.clone())
                .collect()
        };

        for key in due {
            self.revalidate(&key);
        }
    }

    /// Build a cache key from a base key and a parsed filter, so two subscribers
    /// with equivalent filters share one entry.
    ///
    /// `base` comes back unchanged when there is nothing to filter by — `None`,
    /// or a group holding no filters, which is what an empty query parses to.
    /// Otherwise the filter follows a `?` separator in the canonical spelling
    /// [`rusty_filter::canonical_key`] produces, so `[age] > 1` and
    /// `[age] greater than 1` are one key rather than two.
    ///
    /// An associated function, not a method: it reads no cache state, which also
    /// keeps it clear of this module's rule that the cache lock is never held
    /// across an `.await`.
    ///
    /// ```
    /// use rusty::core::QueryService;
    /// use rusty_filter::parse_query_unchecked;
    ///
    /// let a = parse_query_unchecked("[age] > 1").unwrap();
    /// let b = parse_query_unchecked("[age] greater than 1").unwrap();
    /// assert_eq!(
    ///     QueryService::filtered_key("people", Some(&a)),
    ///     QueryService::filtered_key("people", Some(&b)),
    /// );
    /// assert_eq!(QueryService::filtered_key("people", None), "people");
    /// ```
    pub fn filtered_key(base: &str, filter: Option<&rusty_filter::FilterGroup>) -> String {
        match filter {
            Some(filter) if !filter.filters.is_empty() => {
                format!("{base}?{}", rusty_filter::canonical_key(filter))
            }
            _ => base.to_string(),
        }
    }

    /// Transition an entry into `Fetching`/`Revalidating` and return the fetcher
    /// to spawn, or `None` when a fetch is already running.
    fn begin_fetch(entry: &mut QueryEntry, revalidating: bool) -> Option<ErasedFetcher> {
        let fetcher = entry.fetcher.clone()?;

        // Dedup: Ivy shares the in-flight `Task`; an in-process state check is
        // equivalent and avoids sharing a `Future` across tasks.
        if matches!(
            entry.state,
            QueryEntryState::Fetching | QueryEntryState::Revalidating
        ) {
            return None;
        }

        entry.abort_fetch();
        if revalidating {
            entry.state = QueryEntryState::Revalidating;
        } else {
            entry.state = QueryEntryState::Fetching;
            entry.error = None;
        }
        entry.notify();

        Some(fetcher)
    }

    /// Run a fetch on its own task and store its result.
    fn spawn_fetch(self: &Arc<Self>, key: String, fetcher: ErasedFetcher) {
        let service = Arc::clone(self);
        let task_key = key.clone();
        let handle = tokio::spawn(async move {
            let result = (fetcher)().await;
            service.complete_fetch(&task_key, result);
        });

        let mut cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get_mut(&key) {
            entry.fetch_handle = Some(handle);
        } else {
            handle.abort();
        }
    }

    /// Store a completed fetch's outcome and notify subscribers.
    fn complete_fetch(&self, key: &str, result: Result<ErasedValue, QueryError>) {
        let now = Instant::now();
        let mut cache = self.cache.lock().unwrap();
        let Some(entry) = cache.get_mut(key) else {
            return;
        };

        entry.fetch_handle = None;

        match result {
            Ok(value) => {
                entry.value = Some(value);
                entry.error = None;
                entry.state = QueryEntryState::Fresh;
                entry.last_fetched_at = Some(now);
                entry.expires_at = entry.options.expiration.map(|exp| now + exp);
            }
            Err(error) => {
                entry.error = Some(error);
                // Keep any previously fetched value visible, as Ivy does.
                entry.state = if entry.value.is_some() {
                    QueryEntryState::Stale
                } else {
                    QueryEntryState::Empty
                };
            }
        }

        entry.notify();
    }

    fn keys_with_tag(&self, tag: &str) -> Vec<String> {
        let cache = self.cache.lock().unwrap();
        cache
            .iter()
            .filter(|(_, entry)| entry.tags.iter().any(|t| t == tag))
            .map(|(key, _)| key.clone())
            .collect()
    }

    fn keys_where(&self, predicate: impl Fn(&str) -> bool) -> Vec<String> {
        let cache = self.cache.lock().unwrap();
        cache.keys().filter(|key| predicate(key)).cloned().collect()
    }
}

impl Default for QueryService {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for QueryService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryService")
            .field("entries", &self.entry_count())
            .field("options", &self.options)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// One recorded notification: the downcast value, the state and any error.
    type Notification = (Option<String>, QueryEntryState, Option<QueryError>);

    /// Records every notification a subscriber receives.
    #[derive(Default)]
    struct Recorder {
        notifications: Mutex<Vec<Notification>>,
    }

    impl Recorder {
        fn subscriber(self: &Arc<Self>) -> QuerySubscriberFn {
            let recorder = Arc::clone(self);
            Arc::new(move |value, state, error| {
                let typed = value.and_then(|v| v.downcast_ref::<String>().cloned());
                recorder
                    .notifications
                    .lock()
                    .unwrap()
                    .push((typed, state, error));
            })
        }

        fn states(&self) -> Vec<QueryEntryState> {
            self.notifications
                .lock()
                .unwrap()
                .iter()
                .map(|(_, state, _)| *state)
                .collect()
        }

        fn last_value(&self) -> Option<String> {
            self.notifications
                .lock()
                .unwrap()
                .last()
                .and_then(|(value, _, _)| value.clone())
        }

        fn last_error(&self) -> Option<QueryError> {
            self.notifications
                .lock()
                .unwrap()
                .last()
                .and_then(|(_, _, error)| error.clone())
        }
    }

    /// A fetcher returning `value` and counting its calls.
    fn counting_fetcher(value: &str, calls: Arc<AtomicUsize>) -> ErasedFetcher {
        let value = value.to_string();
        erase_fetcher(move || {
            let value = value.clone();
            let calls = calls.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(value)
            }
        })
    }

    fn failing_fetcher(message: &str) -> ErasedFetcher {
        let message = message.to_string();
        erase_fetcher(move || {
            let message = message.clone();
            async move { Err::<String, _>(QueryError::new(message)) }
        })
    }

    /// Let spawned fetch tasks run to completion.
    async fn settle() {
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
    }

    /// Subscribe and immediately register the recorder, mirroring what the hook does.
    fn subscribe_with(
        service: &Arc<QueryService>,
        key: &str,
        recorder: &Arc<Recorder>,
        fetcher: ErasedFetcher,
        options: QueryOptions,
        tags: Vec<String>,
        initial_value: Option<ErasedValue>,
    ) -> (Uuid, QuerySubscription) {
        let id = Uuid::new_v4();
        service.set_subscriber(key, id, recorder.subscriber());
        let subscription = service.subscribe(key, id, fetcher, options, tags, initial_value);
        service.set_subscriber(key, id, recorder.subscriber());
        (id, subscription)
    }

    #[tokio::test]
    async fn test_subscribe_fetches_and_notifies() {
        let service = Arc::new(QueryService::new());
        let recorder = Arc::new(Recorder::default());
        let calls = Arc::new(AtomicUsize::new(0));

        let (_id, _sub) = subscribe_with(
            &service,
            "user:1",
            &recorder,
            counting_fetcher("alice", calls.clone()),
            QueryOptions::default(),
            vec![],
            None,
        );

        settle().await;

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(service.entry_state("user:1"), Some(QueryEntryState::Fresh));
        assert_eq!(recorder.last_value().as_deref(), Some("alice"));
        assert_eq!(*recorder.states().last().unwrap(), QueryEntryState::Fresh);
    }

    #[tokio::test]
    async fn test_second_subscriber_to_fresh_entry_gets_cached_value_without_refetch() {
        let service = Arc::new(QueryService::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let first = Arc::new(Recorder::default());

        // Expiration set => no background revalidation on mount.
        let options = QueryOptions::default().expiration(Duration::from_secs(60));
        let (_id, _sub) = subscribe_with(
            &service,
            "k",
            &first,
            counting_fetcher("v", calls.clone()),
            options.clone(),
            vec![],
            None,
        );
        settle().await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let second = Arc::new(Recorder::default());
        let (_id2, _sub2) = subscribe_with(
            &service,
            "k",
            &second,
            counting_fetcher("v", calls.clone()),
            options,
            vec![],
            None,
        );
        settle().await;

        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "a Fresh entry must not refetch for a second subscriber"
        );
        assert_eq!(second.last_value().as_deref(), Some("v"));
    }

    #[tokio::test]
    async fn test_fresh_entry_revalidates_in_background_on_mount() {
        let service = Arc::new(QueryService::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let first = Arc::new(Recorder::default());

        let (_id, _sub) = subscribe_with(
            &service,
            "k",
            &first,
            counting_fetcher("v", calls.clone()),
            QueryOptions::default(),
            vec![],
            None,
        );
        settle().await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let second = Arc::new(Recorder::default());
        let (_id2, _sub2) = subscribe_with(
            &service,
            "k",
            &second,
            counting_fetcher("v", calls.clone()),
            QueryOptions::default(),
            vec![],
            None,
        );
        settle().await;

        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "SWR revalidates in the background on mount"
        );
        assert!(second.states().contains(&QueryEntryState::Revalidating));
    }

    #[tokio::test]
    async fn test_expiration_moves_fresh_to_stale_and_revalidates() {
        tokio::time::pause();

        let service = Arc::new(QueryService::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let first = Arc::new(Recorder::default());
        let options = QueryOptions::default().expiration(Duration::from_secs(30));

        let (_id, _sub) = subscribe_with(
            &service,
            "k",
            &first,
            counting_fetcher("v", calls.clone()),
            options.clone(),
            vec![],
            None,
        );
        settle().await;
        assert_eq!(service.entry_state("k"), Some(QueryEntryState::Fresh));

        tokio::time::advance(Duration::from_secs(31)).await;

        let second = Arc::new(Recorder::default());
        let (_id2, _sub2) = subscribe_with(
            &service,
            "k",
            &second,
            counting_fetcher("v2", calls.clone()),
            options,
            vec![],
            None,
        );

        // The stale value is delivered immediately, then revalidated.
        assert!(second.states().contains(&QueryEntryState::Stale));
        settle().await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(second.last_value().as_deref(), Some("v2"));
    }

    #[tokio::test]
    async fn test_mutate_without_revalidate_sets_fresh_without_fetching() {
        let service = Arc::new(QueryService::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let recorder = Arc::new(Recorder::default());

        let (_id, _sub) = subscribe_with(
            &service,
            "k",
            &recorder,
            counting_fetcher("v", calls.clone()),
            QueryOptions::default(),
            vec![],
            None,
        );
        settle().await;
        let before = calls.load(Ordering::SeqCst);

        service.mutate("k", Some(Arc::new("optimistic".to_string())), false);
        settle().await;

        assert_eq!(calls.load(Ordering::SeqCst), before, "no refetch expected");
        assert_eq!(service.entry_state("k"), Some(QueryEntryState::Fresh));
        assert_eq!(recorder.last_value().as_deref(), Some("optimistic"));
    }

    #[tokio::test]
    async fn test_mutate_with_revalidate_refetches() {
        let service = Arc::new(QueryService::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let recorder = Arc::new(Recorder::default());

        let (_id, _sub) = subscribe_with(
            &service,
            "k",
            &recorder,
            counting_fetcher("server", calls.clone()),
            QueryOptions::default(),
            vec![],
            None,
        );
        settle().await;
        let before = calls.load(Ordering::SeqCst);

        service.mutate("k", Some(Arc::new("optimistic".to_string())), true);
        assert!(recorder.states().contains(&QueryEntryState::Revalidating));
        settle().await;

        assert_eq!(calls.load(Ordering::SeqCst), before + 1);
        assert_eq!(recorder.last_value().as_deref(), Some("server"));
        assert_eq!(service.entry_state("k"), Some(QueryEntryState::Fresh));
    }

    #[tokio::test]
    async fn test_revalidate_and_invalidate_transitions() {
        let service = Arc::new(QueryService::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let recorder = Arc::new(Recorder::default());
        // Expiration keeps mount from revalidating, isolating the transitions.
        let options = QueryOptions::default().expiration(Duration::from_secs(600));

        let (_id, _sub) = subscribe_with(
            &service,
            "k",
            &recorder,
            counting_fetcher("v", calls.clone()),
            options,
            vec![],
            None,
        );
        settle().await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // revalidate keeps the value visible -> Revalidating
        service.revalidate("k");
        assert_eq!(
            service.entry_state("k"),
            Some(QueryEntryState::Revalidating)
        );
        settle().await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(service.peek::<String>("k").as_deref(), Some("v"));

        // invalidate drops the value -> Fetching (there is still a subscriber)
        service.invalidate("k");
        assert_eq!(service.entry_state("k"), Some(QueryEntryState::Fetching));
        assert_eq!(service.peek::<String>("k"), None);
        settle().await;
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(service.entry_state("k"), Some(QueryEntryState::Fresh));
    }

    #[tokio::test]
    async fn test_fetch_error_sets_error_and_leaves_prior_value_stale() {
        let service = Arc::new(QueryService::new());
        let recorder = Arc::new(Recorder::default());
        let calls = Arc::new(AtomicUsize::new(0));
        let options = QueryOptions::default().expiration(Duration::from_secs(600));

        let (_id, _sub) = subscribe_with(
            &service,
            "k",
            &recorder,
            counting_fetcher("good", calls.clone()),
            options,
            vec![],
            None,
        );
        settle().await;
        assert_eq!(service.peek::<String>("k").as_deref(), Some("good"));

        // Swap in a failing fetcher and force a revalidation.
        service.set_fetcher("k", failing_fetcher("boom"));
        service.revalidate("k");
        settle().await;

        assert_eq!(service.entry_state("k"), Some(QueryEntryState::Stale));
        assert_eq!(
            service.peek::<String>("k").as_deref(),
            Some("good"),
            "the prior value stays visible"
        );
        assert_eq!(recorder.last_error(), Some(QueryError::new("boom")));
    }

    #[tokio::test]
    async fn test_error_on_empty_entry_leaves_it_empty() {
        let service = Arc::new(QueryService::new());
        let recorder = Arc::new(Recorder::default());

        let (_id, _sub) = subscribe_with(
            &service,
            "k",
            &recorder,
            failing_fetcher("nope"),
            QueryOptions::default(),
            vec![],
            None,
        );
        settle().await;

        assert_eq!(service.entry_state("k"), Some(QueryEntryState::Empty));
        assert_eq!(recorder.last_error(), Some(QueryError::new("nope")));
    }

    #[tokio::test]
    async fn test_dropping_last_subscription_aborts_inflight_fetch() {
        let service = Arc::new(QueryService::new());
        let recorder = Arc::new(Recorder::default());
        let completed = Arc::new(AtomicUsize::new(0));

        let slow_fetcher = {
            let completed = completed.clone();
            erase_fetcher(move || {
                let completed = completed.clone();
                async move {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    completed.fetch_add(1, Ordering::SeqCst);
                    Ok("late".to_string())
                }
            })
        };

        let (_id, subscription) = subscribe_with(
            &service,
            "k",
            &recorder,
            slow_fetcher,
            QueryOptions::default(),
            vec![],
            None,
        );
        settle().await;
        assert_eq!(service.entry_state("k"), Some(QueryEntryState::Fetching));

        drop(subscription);
        assert_eq!(
            service.entry_state("k"),
            Some(QueryEntryState::Empty),
            "an aborted fetch with no value rolls back to Empty"
        );

        tokio::time::pause();
        tokio::time::advance(Duration::from_secs(30)).await;
        settle().await;
        assert_eq!(
            completed.load(Ordering::SeqCst),
            0,
            "the fetch task should have been aborted"
        );
    }

    #[tokio::test]
    async fn test_invalidate_and_revalidate_by_tag_hit_only_tagged_entries() {
        let service = Arc::new(QueryService::new());
        let tagged_calls = Arc::new(AtomicUsize::new(0));
        let untagged_calls = Arc::new(AtomicUsize::new(0));
        let options = QueryOptions::default().expiration(Duration::from_secs(600));

        let r1 = Arc::new(Recorder::default());
        let (_i1, _s1) = subscribe_with(
            &service,
            "users:1",
            &r1,
            counting_fetcher("a", tagged_calls.clone()),
            options.clone(),
            vec!["users".to_string()],
            None,
        );
        let r2 = Arc::new(Recorder::default());
        let (_i2, _s2) = subscribe_with(
            &service,
            "posts:1",
            &r2,
            counting_fetcher("b", untagged_calls.clone()),
            options,
            vec!["posts".to_string()],
            None,
        );
        settle().await;
        assert_eq!(tagged_calls.load(Ordering::SeqCst), 1);
        assert_eq!(untagged_calls.load(Ordering::SeqCst), 1);

        service.revalidate_by_tag("users");
        settle().await;
        assert_eq!(tagged_calls.load(Ordering::SeqCst), 2);
        assert_eq!(untagged_calls.load(Ordering::SeqCst), 1);

        service.invalidate_by_tag("posts");
        settle().await;
        assert_eq!(tagged_calls.load(Ordering::SeqCst), 2);
        assert_eq!(untagged_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_invalidate_where_matches_by_key() {
        let service = Arc::new(QueryService::new());
        let a_calls = Arc::new(AtomicUsize::new(0));
        let b_calls = Arc::new(AtomicUsize::new(0));
        let options = QueryOptions::default().expiration(Duration::from_secs(600));

        let r1 = Arc::new(Recorder::default());
        let (_i1, _s1) = subscribe_with(
            &service,
            "user:1",
            &r1,
            counting_fetcher("a", a_calls.clone()),
            options.clone(),
            vec![],
            None,
        );
        let r2 = Arc::new(Recorder::default());
        let (_i2, _s2) = subscribe_with(
            &service,
            "post:1",
            &r2,
            counting_fetcher("b", b_calls.clone()),
            options,
            vec![],
            None,
        );
        settle().await;

        service.revalidate_where(|key| key.starts_with("user:"));
        settle().await;
        assert_eq!(a_calls.load(Ordering::SeqCst), 2);
        assert_eq!(b_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_eviction_drops_orphaned_expired_and_empty_entries() {
        tokio::time::pause();

        let service = Arc::new(QueryService::new());
        let calls = Arc::new(AtomicUsize::new(0));

        // An expired, orphaned entry.
        {
            let recorder = Arc::new(Recorder::default());
            let (_id, sub) = subscribe_with(
                &service,
                "expired",
                &recorder,
                counting_fetcher("v", calls.clone()),
                QueryOptions::default().expiration(Duration::from_secs(1)),
                vec![],
                None,
            );
            settle().await;
            drop(sub);
        }

        // An orphaned entry whose fetch never produced a value stays Empty.
        {
            let recorder = Arc::new(Recorder::default());
            let (_id, sub) = subscribe_with(
                &service,
                "empty",
                &recorder,
                failing_fetcher("no"),
                QueryOptions::default(),
                vec![],
                None,
            );
            settle().await;
            drop(sub);
        }
        assert_eq!(service.entry_count(), 2);

        tokio::time::advance(Duration::from_secs(5)).await;
        service.evict_expired_entries();
        assert_eq!(
            service.entry_count(),
            0,
            "expired and Empty orphans should be evicted"
        );
    }

    #[tokio::test]
    async fn test_eviction_keeps_subscribed_entries() {
        tokio::time::pause();

        let service = Arc::new(QueryService::new());
        let recorder = Arc::new(Recorder::default());
        let calls = Arc::new(AtomicUsize::new(0));

        let (_id, _sub) = subscribe_with(
            &service,
            "live",
            &recorder,
            counting_fetcher("v", calls.clone()),
            QueryOptions::default().expiration(Duration::from_secs(1)),
            vec![],
            None,
        );
        settle().await;

        tokio::time::advance(Duration::from_secs(60 * 61)).await;
        service.evict_expired_entries();
        assert_eq!(
            service.entry_count(),
            1,
            "an entry with subscribers is never evicted"
        );
    }

    #[tokio::test]
    async fn test_lru_eviction_respects_max_entries() {
        tokio::time::pause();

        let service = Arc::new(QueryService::with_options(QueryServiceOptions {
            max_entries: Some(2),
            ..Default::default()
        }));
        let calls = Arc::new(AtomicUsize::new(0));

        // Three orphaned Fresh entries with distinct access times.
        for key in ["oldest", "middle", "newest"] {
            let recorder = Arc::new(Recorder::default());
            let (_id, sub) = subscribe_with(
                &service,
                key,
                &recorder,
                counting_fetcher("v", calls.clone()),
                // No expiration and no mount revalidation keeps them Fresh, so only
                // the LRU rule can evict them.
                QueryOptions::default().revalidate_on_mount(false),
                vec![],
                Some(Arc::new("seed".to_string())),
            );
            settle().await;
            drop(sub);
            tokio::time::advance(Duration::from_secs(10)).await;
        }
        assert_eq!(service.entry_count(), 3);

        service.evict_expired_entries();
        assert_eq!(service.entry_count(), 2);
        assert!(
            service.entry_state("oldest").is_none(),
            "the least recently accessed orphan should be evicted first"
        );
        assert!(service.entry_state("newest").is_some());
    }

    #[tokio::test]
    async fn test_concurrent_subscribes_start_exactly_one_fetch() {
        let service = Arc::new(QueryService::new());
        let calls = Arc::new(AtomicUsize::new(0));

        let slow_fetcher = {
            let calls = calls.clone();
            erase_fetcher(move || {
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Ok("v".to_string())
                }
            })
        };

        let mut handles = Vec::new();
        for _ in 0..8 {
            let service = Arc::clone(&service);
            let fetcher = slow_fetcher.clone();
            handles.push(tokio::spawn(async move {
                let recorder = Arc::new(Recorder::default());
                let (_id, sub) = subscribe_with(
                    &service,
                    "shared",
                    &recorder,
                    fetcher,
                    QueryOptions::default(),
                    vec![],
                    None,
                );
                // Hold the subscription so the fetch is not aborted as orphaned.
                tokio::time::sleep(Duration::from_millis(200)).await;
                drop(sub);
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "concurrent subscribers to one key must share a single fetch"
        );
    }

    #[tokio::test]
    async fn test_refresh_interval_revalidates_due_entries() {
        tokio::time::pause();

        let service = Arc::new(QueryService::new());
        let recorder = Arc::new(Recorder::default());
        let calls = Arc::new(AtomicUsize::new(0));

        let (_id, _sub) = subscribe_with(
            &service,
            "k",
            &recorder,
            counting_fetcher("v", calls.clone()),
            QueryOptions::default()
                .expiration(Duration::from_secs(600))
                .refresh_interval(Duration::from_secs(30)),
            vec![],
            None,
        );
        settle().await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // Not yet due.
        service.refresh_due_entries();
        settle().await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        tokio::time::advance(Duration::from_secs(31)).await;
        service.refresh_due_entries();
        settle().await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_revalidate_on_mount_false_with_initial_value_skips_fetch() {
        let service = Arc::new(QueryService::new());
        let recorder = Arc::new(Recorder::default());
        let calls = Arc::new(AtomicUsize::new(0));

        let (_id, _sub) = subscribe_with(
            &service,
            "k",
            &recorder,
            counting_fetcher("fetched", calls.clone()),
            QueryOptions::default().revalidate_on_mount(false),
            vec![],
            Some(Arc::new("seeded".to_string())),
        );
        settle().await;

        assert_eq!(calls.load(Ordering::SeqCst), 0, "no fetch expected");
        assert_eq!(service.entry_state("k"), Some(QueryEntryState::Fresh));
        assert_eq!(service.peek::<String>("k").as_deref(), Some("seeded"));
    }

    #[tokio::test]
    async fn test_clear_invalidates_every_entry() {
        let service = Arc::new(QueryService::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let options = QueryOptions::default().expiration(Duration::from_secs(600));

        let r1 = Arc::new(Recorder::default());
        let (_i1, s1) = subscribe_with(
            &service,
            "a",
            &r1,
            counting_fetcher("a", calls.clone()),
            options.clone(),
            vec![],
            None,
        );
        let r2 = Arc::new(Recorder::default());
        let (_i2, s2) = subscribe_with(
            &service,
            "b",
            &r2,
            counting_fetcher("b", calls.clone()),
            options,
            vec![],
            None,
        );
        settle().await;
        drop(s1);
        drop(s2);

        service.clear();
        settle().await;

        assert_eq!(service.entry_state("a"), Some(QueryEntryState::Empty));
        assert_eq!(service.entry_state("b"), Some(QueryEntryState::Empty));
        assert_eq!(service.peek::<String>("a"), None);
    }

    fn filter(query: &str) -> rusty_filter::FilterGroup {
        rusty_filter::parse_query_unchecked(query).expect("valid query")
    }

    #[test]
    fn test_filtered_key_returns_the_base_when_there_is_no_filter() {
        assert_eq!(QueryService::filtered_key("people", None), "people");
        // An empty query parses to an empty group, so a cleared filter box must
        // land back on the unfiltered entry instead of creating `people?`.
        assert_eq!(
            QueryService::filtered_key("people", Some(&filter(""))),
            "people"
        );
        assert_eq!(
            QueryService::filtered_key("people", Some(&rusty_filter::FilterGroup::default())),
            "people"
        );
    }

    #[test]
    fn test_filtered_key_appends_the_canonical_filter() {
        // The canonical spelling is the printer's, which is `>` rather than
        // `greater than` — so the key does not echo the query as written.
        assert_eq!(
            QueryService::filtered_key("people", Some(&filter("[age] greater than 1"))),
            "people?[age] > 1"
        );
        assert_eq!(
            QueryService::filtered_key("people", Some(&filter(r#"[name] CONTAINS "a""#))),
            r#"people?[name] contains "a""#
        );
    }

    #[test]
    fn test_equivalent_filters_share_one_key() {
        // The point of keying on the canonical spelling: two subscribers who
        // wrote the same filter differently must not each get their own fetch.
        for [a, b] in [
            ["[age] > 1", "[age] greater than 1"],
            ["[age] = 7", "[age] = 007"],
            ["[age] >= 1", "[age] GREATER THAN OR EQUAL 1"],
        ] {
            assert_eq!(
                QueryService::filtered_key("people", Some(&filter(a))),
                QueryService::filtered_key("people", Some(&filter(b))),
                "{a:?} vs {b:?}"
            );
        }
    }

    #[test]
    fn test_different_filters_get_different_keys() {
        let keys = [
            QueryService::filtered_key("people", None),
            QueryService::filtered_key("people", Some(&filter("[age] > 1"))),
            QueryService::filtered_key("people", Some(&filter("[age] > 2"))),
            QueryService::filtered_key("people", Some(&filter("[age] >= 1"))),
            QueryService::filtered_key("other", Some(&filter("[age] > 1"))),
        ];
        let mut unique = keys.to_vec();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), keys.len(), "keys collided: {keys:?}");
    }
}
