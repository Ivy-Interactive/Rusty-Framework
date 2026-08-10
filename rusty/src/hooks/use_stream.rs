use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{Stream, StreamExt};
use tokio::task::JoinHandle;

use crate::core::query_cache::QueryError;
use crate::hooks::deps::DynEq;
use crate::hooks::use_effect::use_effect_with_deps;
use crate::hooks::use_ref::use_ref;
use crate::hooks::use_state::{use_state, State};
use crate::views::view::BuildContext;

/// Where a consumed stream is in its lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum StreamStatus {
    /// Nothing is running: either `auto_start: false` and no `restart` yet, or
    /// [`StreamResult::stop`] was called.
    #[default]
    Idle,
    /// Chunks are arriving. A retry stays `Streaming` — a recovered hiccup is not
    /// something a view should have to render.
    Streaming,
    /// The stream ended without an error.
    Done,
    /// The stream failed and every retry was used up.
    Error(String),
}

/// How [`use_stream`] consumes its stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamOptions {
    /// Open the stream on mount. `false` waits for [`StreamResult::restart`].
    pub auto_start: bool,
    /// Reopen the stream this many times after a failure. `0` means give up on the
    /// first error.
    pub max_retries: u32,
    /// How long to wait before each retry.
    pub retry_delay: Duration,
    /// Cap how many chunks are retained, oldest dropped first. `None` keeps all of
    /// them — fine for an LLM response, wrong for an endless feed.
    pub max_chunks: Option<usize>,
}

impl Default for StreamOptions {
    fn default() -> Self {
        StreamOptions {
            auto_start: true,
            max_retries: 0,
            retry_delay: Duration::from_secs(1),
            max_chunks: None,
        }
    }
}

impl StreamOptions {
    pub fn new() -> Self {
        StreamOptions::default()
    }

    pub fn auto_start(mut self, auto_start: bool) -> Self {
        self.auto_start = auto_start;
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn retry_delay(mut self, retry_delay: Duration) -> Self {
        self.retry_delay = retry_delay;
        self
    }

    pub fn max_chunks(mut self, max_chunks: usize) -> Self {
        self.max_chunks = Some(max_chunks);
        self
    }
}

/// The chunks a view has received so far, plus the controls to stop and reopen.
///
/// Cheap to clone — every field is `Arc`-backed, so an event handler can capture it.
pub struct StreamResult<T: Send + Sync + Clone + 'static> {
    /// Chunks in arrival order, capped by
    /// [`StreamOptions::max_chunks`](StreamOptions#structfield.max_chunks).
    pub chunks: State<Vec<T>>,
    pub status: State<StreamStatus>,
    restart: Arc<dyn Fn() + Send + Sync>,
    stop: Arc<dyn Fn() + Send + Sync>,
}

impl<T: Send + Sync + Clone + 'static> StreamResult<T> {
    /// Abort whatever is running, drop the chunks, reset the status to `Idle`, and
    /// reopen the stream from the factory. Also how an `auto_start: false` stream is
    /// started.
    pub fn restart(&self) {
        (self.restart)();
    }

    /// Abort whatever is running and go back to `Idle`, keeping the chunks received
    /// so far.
    pub fn stop(&self) {
        (self.stop)();
    }

    pub fn is_streaming(&self) -> bool {
        self.status.get() == StreamStatus::Streaming
    }

    /// Number of chunks received so far.
    pub fn len(&self) -> usize {
        self.chunks.get().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T: Send + Sync + Clone + 'static> Clone for StreamResult<T> {
    fn clone(&self) -> Self {
        StreamResult {
            chunks: self.chunks.clone(),
            status: self.status.clone(),
            restart: Arc::clone(&self.restart),
            stop: Arc::clone(&self.stop),
        }
    }
}

impl<T: Send + Sync + Clone + 'static + std::fmt::Debug> std::fmt::Debug for StreamResult<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamResult")
            .field("chunks", &self.chunks.get())
            .field("status", &self.status.get())
            .finish()
    }
}

/// The concatenated text of a token stream, handed back by [`use_stream_text`].
pub struct TextStreamResult {
    /// Every chunk so far, joined in arrival order.
    pub text: State<String>,
    pub status: State<StreamStatus>,
    restart: Arc<dyn Fn() + Send + Sync>,
    stop: Arc<dyn Fn() + Send + Sync>,
}

impl TextStreamResult {
    /// Abort whatever is running, clear the text, reset the status to `Idle`, and
    /// reopen the stream.
    pub fn restart(&self) {
        (self.restart)();
    }

    /// Abort whatever is running and go back to `Idle`, keeping the text so far.
    pub fn stop(&self) {
        (self.stop)();
    }

    pub fn is_streaming(&self) -> bool {
        self.status.get() == StreamStatus::Streaming
    }
}

impl Clone for TextStreamResult {
    fn clone(&self) -> Self {
        TextStreamResult {
            text: self.text.clone(),
            status: self.status.clone(),
            restart: Arc::clone(&self.restart),
            stop: Arc::clone(&self.stop),
        }
    }
}

impl std::fmt::Debug for TextStreamResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextStreamResult")
            .field("text", &self.text.get())
            .field("status", &self.status.get())
            .finish()
    }
}

/// The task slot a stream hook keeps across builds.
type TaskSlot = Arc<Mutex<Option<JoinHandle<()>>>>;

/// Consume a `futures::Stream` into view state, rebuilding as chunks arrive.
///
/// The counterpart to [`use_download_stream`](crate::hooks::use_download_stream),
/// which streams *out* to the client. This one streams *in* from a server-side
/// source — an LLM SDK call, an SSE relay, a channel feed — and pushes each chunk
/// to the browser over the session's WebSocket.
///
/// `factory` returns the stream, so a retry or a
/// [`restart`](StreamResult::restart) can reopen it: an exhausted `Stream` cannot
/// be rewound, and this is the same shape `use_download_stream` uses.
///
/// ```ignore
/// let tokens = use_stream(ctx, || async { open_llm_stream(&prompt).await }, StreamOptions::new());
/// TextBlock::new(&tokens.chunks.get().join(""))
/// ```
pub fn use_stream<T, F, Fut, S>(
    ctx: &mut BuildContext,
    factory: F,
    options: StreamOptions,
) -> StreamResult<T>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<S, QueryError>> + Send + 'static,
    S: Stream<Item = Result<T, QueryError>> + Send + 'static,
    T: Send + Sync + Clone + 'static,
{
    let chunks = use_state(ctx, Vec::<T>::new());
    let status = use_state(ctx, StreamStatus::default());
    // A `use_state`, not a `use_ref`: `restart` bumps it, and only a
    // rebuild-triggering set makes the next build re-register the effect.
    let generation = use_state(ctx, 0u64);
    let task: TaskSlot = use_ref(ctx, Arc::new(Mutex::new(None))).get();

    let on_chunk = {
        let chunks = chunks.clone();
        let max_chunks = options.max_chunks;
        Arc::new(move |chunk: T| {
            chunks.update(move |previous| {
                let mut next = previous.clone();
                next.push(chunk);
                if let Some(cap) = max_chunks {
                    while next.len() > cap {
                        next.remove(0);
                    }
                }
                next
            });
        }) as Arc<dyn Fn(T) + Send + Sync>
    };

    let clear = {
        let chunks = chunks.clone();
        Arc::new(move || chunks.set(Vec::new())) as Arc<dyn Fn() + Send + Sync>
    };

    register_stream_effect(ctx, factory, options, &status, &generation, &task, on_chunk);

    StreamResult {
        chunks,
        status: status.clone(),
        restart: restart_fn(&status, &generation, &task, clear),
        stop: stop_fn(&status, &task),
    }
}

/// [`use_stream`] for token streams: concatenates the chunks instead of collecting
/// them, which is what an LLM response is rendered from.
///
/// `max_chunks` caps how many chunks are *appended*; later chunks are still drained
/// from the stream but not shown, so a runaway generator cannot grow the string
/// without bound.
pub fn use_stream_text<F, Fut, S>(
    ctx: &mut BuildContext,
    factory: F,
    options: StreamOptions,
) -> TextStreamResult
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<S, QueryError>> + Send + 'static,
    S: Stream<Item = Result<String, QueryError>> + Send + 'static,
{
    let text = use_state(ctx, String::new());
    let status = use_state(ctx, StreamStatus::default());
    let generation = use_state(ctx, 0u64);
    let task: TaskSlot = use_ref(ctx, Arc::new(Mutex::new(None))).get();

    let appended = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let on_chunk = {
        let text = text.clone();
        let appended = Arc::clone(&appended);
        let max_chunks = options.max_chunks;
        Arc::new(move |chunk: String| {
            let seen = appended.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if max_chunks.is_some_and(|cap| seen >= cap) {
                return;
            }
            text.update(move |previous| format!("{previous}{chunk}"));
        }) as Arc<dyn Fn(String) + Send + Sync>
    };

    let clear = {
        let text = text.clone();
        let appended = Arc::clone(&appended);
        Arc::new(move || {
            appended.store(0, std::sync::atomic::Ordering::SeqCst);
            text.set(String::new());
        }) as Arc<dyn Fn() + Send + Sync>
    };

    register_stream_effect(ctx, factory, options, &status, &generation, &task, on_chunk);

    TextStreamResult {
        text,
        status: status.clone(),
        restart: restart_fn(&status, &generation, &task, clear),
        stop: stop_fn(&status, &task),
    }
}

/// Register the one effect both hooks use, keyed on `generation` so a `restart`
/// reopens the stream.
///
/// The effect is the last slot either hook consumes, so the two share a slot layout
/// and swapping one for the other at a call site shifts nothing.
fn register_stream_effect<T, F, Fut, S>(
    ctx: &mut BuildContext,
    factory: F,
    options: StreamOptions,
    status: &State<StreamStatus>,
    generation: &State<u64>,
    task: &TaskSlot,
    on_chunk: Arc<dyn Fn(T) + Send + Sync>,
) where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<S, QueryError>> + Send + 'static,
    S: Stream<Item = Result<T, QueryError>> + Send + 'static,
    T: Send + Sync + Clone + 'static,
{
    let current = generation.get();
    let status = status.clone();
    let spawn_slot = Arc::clone(task);
    let cleanup_slot = Arc::clone(task);

    use_effect_with_deps(ctx, &[&current as &dyn DynEq], move |_| {
        // Generation 0 with `auto_start: false` means the view has not asked for
        // anything yet: stay Idle and never call the factory.
        if current == 0 && !options.auto_start {
            return None;
        }

        // Belt and braces: the runtime runs the previous effect's cleanup before
        // this callback, but a `restart` that raced a rebuild must not leave two
        // tasks writing the same state.
        abort_previous(&spawn_slot);
        let handle = spawn_stream_task(factory, options, status, on_chunk);
        *spawn_slot.lock().unwrap() = Some(handle);

        Some(Box::new(move || {
            abort_previous(&cleanup_slot);
        }) as Box<dyn FnOnce() + Send + Sync>)
    });
}

/// Drive the stream on its own task, retrying a failure per [`StreamOptions`].
fn spawn_stream_task<T, F, Fut, S>(
    factory: F,
    options: StreamOptions,
    status: State<StreamStatus>,
    on_chunk: Arc<dyn Fn(T) + Send + Sync>,
) -> JoinHandle<()>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<S, QueryError>> + Send + 'static,
    S: Stream<Item = Result<T, QueryError>> + Send + 'static,
    T: Send + Sync + Clone + 'static,
{
    tokio::spawn(async move {
        let mut retries_used = 0u32;

        loop {
            status.set(StreamStatus::Streaming);

            // A failure — opening the stream or a chunk part-way through — is
            // whatever we end this attempt with. Chunks already delivered stay.
            let failure = match factory().await {
                Err(error) => Some(error),
                Ok(stream) => {
                    let mut stream = Box::pin(stream);
                    let mut failure = None;
                    while let Some(item) = stream.next().await {
                        match item {
                            Ok(chunk) => on_chunk(chunk),
                            Err(error) => {
                                failure = Some(error);
                                break;
                            }
                        }
                    }
                    failure
                }
            };

            let Some(error) = failure else {
                status.set(StreamStatus::Done);
                return;
            };

            if retries_used >= options.max_retries {
                status.set(StreamStatus::Error(error.message));
                return;
            }
            retries_used += 1;
            tokio::time::sleep(options.retry_delay).await;
        }
    })
}

/// Abort the task in `slot`, if any. Safe to call when nothing is running.
fn abort_previous(slot: &TaskSlot) {
    if let Some(handle) = slot.lock().unwrap().take() {
        handle.abort();
    }
}

/// `restart`: abort now (rather than waiting for the effect's cleanup), clear the
/// accumulated output, and bump the generation so the next build reopens.
///
/// The status goes back to `Idle` as part of this: leaving a `Done` or `Error` from
/// the previous run in place would make a restart invisible until the first new
/// chunk arrived.
fn restart_fn(
    status: &State<StreamStatus>,
    generation: &State<u64>,
    task: &TaskSlot,
    clear: Arc<dyn Fn() + Send + Sync>,
) -> Arc<dyn Fn() + Send + Sync> {
    let status = status.clone();
    let generation = generation.clone();
    let task = Arc::clone(task);
    Arc::new(move || {
        // Abort first: a task still appending would otherwise race the clear.
        abort_previous(&task);
        clear();
        status.set(StreamStatus::Idle);
        generation.update(|current| current.wrapping_add(1));
    })
}

/// `stop`: abort the task and report `Idle`, keeping whatever arrived.
fn stop_fn(status: &State<StreamStatus>, task: &TaskSlot) -> Arc<dyn Fn() + Send + Sync> {
    let status = status.clone();
    let task = Arc::clone(task);
    Arc::new(move || {
        abort_previous(&task);
        status.set(StreamStatus::Idle);
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::EffectCleanup;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// One build of `hook`, with its effects run and their cleanups collected.
    fn mount<R>(
        store: &mut HookStore,
        hook: impl FnOnce(&mut BuildContext) -> R,
    ) -> (R, Vec<EffectCleanup>) {
        let (result, effects) = {
            let mut ctx = BuildContext::new(store, None);
            let result = hook(&mut ctx);
            (result, ctx.drain_effects())
        };

        let mut cleanups = Vec::new();
        for effect in effects {
            if let Some(cleanup) = (effect.callback)() {
                cleanups.push(cleanup);
            }
        }
        (result, cleanups)
    }

    /// A rebuild of `hook` against the same store, as the runtime does after a
    /// `State::set` — including running the previous effect's cleanup first.
    fn rebuild<R>(
        store: &mut HookStore,
        cleanups: Vec<EffectCleanup>,
        hook: impl FnOnce(&mut BuildContext) -> R,
    ) -> (R, Vec<EffectCleanup>) {
        for cleanup in cleanups {
            cleanup();
        }
        mount(store, hook)
    }

    /// Let every ready task run to its next await point, without moving the clock.
    ///
    /// The paused-clock tests need this instead of [`wait_until`]: a `sleep` on the
    /// test's own task keeps the runtime busy, so tokio's auto-advance never fires
    /// and only an explicit `tokio::time::advance` moves virtual time.
    async fn settle() {
        for _ in 0..32 {
            tokio::task::yield_now().await;
        }
    }

    /// Poll until `predicate` holds, or fail. Only for tests on the real clock —
    /// see [`settle`] for the paused ones.
    async fn wait_until(what: &str, predicate: impl Fn() -> bool) {
        for _ in 0..400 {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("timed out waiting for {what}");
    }

    async fn wait_for_status(status: &State<StreamStatus>, expected: StreamStatus) {
        let probe = status.clone();
        let target = expected.clone();
        wait_until(&format!("status {expected:?}"), move || {
            probe.get() == target
        })
        .await;
    }

    /// A stream that yields `items` one at a time, sleeping between them so a test
    /// can interleave with it.
    fn paced_stream(
        items: Vec<i32>,
        delay: Duration,
    ) -> impl Stream<Item = Result<i32, QueryError>> + Send {
        futures::stream::unfold(items.into_iter(), move |mut items| async move {
            tokio::time::sleep(delay).await;
            let next = items.next();
            next.map(|item| (Ok(item), items))
        })
    }

    #[tokio::test]
    async fn test_chunks_accumulate_in_order_and_status_reaches_done() {
        let mut store = HookStore::new();

        let (result, _cleanups) = mount(&mut store, |ctx| {
            use_stream(
                ctx,
                || async { Ok(futures::stream::iter(vec![Ok(1), Ok(2), Ok(3)])) },
                StreamOptions::new(),
            )
        });

        wait_for_status(&result.status, StreamStatus::Done).await;
        assert_eq!(result.chunks.get(), vec![1, 2, 3]);
        assert_eq!(result.len(), 3);
        assert!(!result.is_empty());
        assert!(!result.is_streaming());
    }

    #[tokio::test]
    async fn test_status_is_streaming_while_chunks_are_still_arriving() {
        let mut store = HookStore::new();

        let (result, _cleanups) = mount(&mut store, |ctx| {
            use_stream(
                ctx,
                || async { Ok(paced_stream(vec![1, 2, 3], Duration::from_millis(20))) },
                StreamOptions::new(),
            )
        });

        let chunks = result.chunks.clone();
        wait_until("the first chunk", move || !chunks.get().is_empty()).await;
        assert!(result.is_streaming(), "got {:?}", result.status.get());

        wait_for_status(&result.status, StreamStatus::Done).await;
        assert_eq!(result.chunks.get(), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_factory_error_with_no_retries_becomes_error() {
        let mut store = HookStore::new();
        let calls = Arc::new(AtomicUsize::new(0));

        let (result, _cleanups) = {
            let calls = Arc::clone(&calls);
            mount(&mut store, move |ctx| {
                use_stream(
                    ctx,
                    move || {
                        let calls = Arc::clone(&calls);
                        async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            Err::<futures::stream::Empty<Result<i32, QueryError>>, _>(
                                QueryError::new("upstream is down"),
                            )
                        }
                    },
                    StreamOptions::new(),
                )
            })
        };

        wait_for_status(
            &result.status,
            StreamStatus::Error("upstream is down".to_string()),
        )
        .await;
        assert_eq!(calls.load(Ordering::SeqCst), 1, "max_retries: 0 means once");
        assert!(result.chunks.get().is_empty());
    }

    #[tokio::test]
    async fn test_a_chunk_error_is_retried_and_keeps_the_earlier_chunks() {
        let mut store = HookStore::new();
        let attempts = Arc::new(AtomicUsize::new(0));

        let (result, _cleanups) = {
            let attempts = Arc::clone(&attempts);
            mount(&mut store, move |ctx| {
                use_stream(
                    ctx,
                    move || {
                        let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                        async move {
                            if attempt == 0 {
                                // Attempt 1 delivers a chunk, then breaks.
                                Ok(futures::stream::iter(vec![
                                    Ok(1),
                                    Err(QueryError::new("connection reset")),
                                    Ok(99),
                                ]))
                            } else {
                                Ok(futures::stream::iter(vec![Ok(2), Ok(3)]))
                            }
                        }
                    },
                    StreamOptions::new()
                        .max_retries(1)
                        .retry_delay(Duration::from_millis(10)),
                )
            })
        };

        wait_for_status(&result.status, StreamStatus::Done).await;
        assert_eq!(
            result.chunks.get(),
            vec![1, 2, 3],
            "the recovered attempt appends to what attempt 1 delivered"
        );
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_retries_are_exhausted_then_report_the_last_error() {
        let mut store = HookStore::new();
        let attempts = Arc::new(AtomicUsize::new(0));

        let (result, _cleanups) = {
            let attempts = Arc::clone(&attempts);
            mount(&mut store, move |ctx| {
                use_stream(
                    ctx,
                    move || {
                        let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                        async move {
                            Err::<futures::stream::Empty<Result<i32, QueryError>>, _>(
                                QueryError::new(format!("attempt {attempt} failed")),
                            )
                        }
                    },
                    StreamOptions::new()
                        .max_retries(2)
                        .retry_delay(Duration::from_millis(10)),
                )
            })
        };

        wait_for_status(
            &result.status,
            StreamStatus::Error("attempt 2 failed".to_string()),
        )
        .await;
        assert_eq!(
            attempts.load(Ordering::SeqCst),
            3,
            "one attempt plus two retries"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn test_a_retry_waits_out_the_retry_delay() {
        let mut store = HookStore::new();
        let attempts = Arc::new(AtomicUsize::new(0));

        let (result, _cleanups) = {
            let attempts = Arc::clone(&attempts);
            mount(&mut store, move |ctx| {
                use_stream(
                    ctx,
                    move || {
                        attempts.fetch_add(1, Ordering::SeqCst);
                        async move {
                            Err::<futures::stream::Empty<Result<i32, QueryError>>, _>(
                                QueryError::new("still down"),
                            )
                        }
                    },
                    StreamOptions::new()
                        .max_retries(1)
                        .retry_delay(Duration::from_secs(10)),
                )
            })
        };

        settle().await;
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        assert_eq!(
            result.status.get(),
            StreamStatus::Streaming,
            "a pending retry is not an Error yet"
        );

        tokio::time::advance(Duration::from_secs(9)).await;
        settle().await;
        assert_eq!(
            attempts.load(Ordering::SeqCst),
            1,
            "the retry must wait out retry_delay, not fire immediately"
        );

        tokio::time::advance(Duration::from_secs(2)).await;
        settle().await;
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        assert_eq!(
            result.status.get(),
            StreamStatus::Error("still down".to_string()),
            "the second failure exhausts the single retry"
        );
    }

    #[tokio::test]
    async fn test_effect_cleanup_aborts_the_task_and_stops_the_writes() {
        let mut store = HookStore::new();

        let (result, cleanups) = mount(&mut store, |ctx| {
            use_stream(
                ctx,
                || async {
                    Ok(paced_stream(
                        (0..100).collect::<Vec<_>>(),
                        Duration::from_millis(10),
                    ))
                },
                StreamOptions::new(),
            )
        });

        let chunks = result.chunks.clone();
        wait_until("the first chunk", move || !chunks.get().is_empty()).await;

        // Unmount.
        for cleanup in cleanups {
            cleanup();
        }
        let at_unmount = result.chunks.get().len();

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(
            result.chunks.get().len(),
            at_unmount,
            "an aborted task must not keep writing state"
        );
        assert_ne!(result.status.get(), StreamStatus::Done);
    }

    #[tokio::test]
    async fn test_restart_clears_the_chunks_and_reopens_the_stream() {
        let mut store = HookStore::new();
        let calls = Arc::new(AtomicUsize::new(0));

        let hook = {
            let calls = Arc::clone(&calls);
            move |ctx: &mut BuildContext| {
                let calls = Arc::clone(&calls);
                use_stream(
                    ctx,
                    move || {
                        let calls = Arc::clone(&calls);
                        async move {
                            let call = calls.fetch_add(1, Ordering::SeqCst);
                            Ok(futures::stream::iter(vec![Ok(call as i32)]))
                        }
                    },
                    StreamOptions::new(),
                )
            }
        };

        let (result, cleanups) = mount(&mut store, hook.clone());
        wait_for_status(&result.status, StreamStatus::Done).await;
        assert_eq!(result.chunks.get(), vec![0]);

        result.restart();
        assert!(
            result.chunks.get().is_empty(),
            "restart clears immediately, before the rebuild"
        );

        // The runtime rebuilds because `generation` changed.
        let (result, _cleanups) = rebuild(&mut store, cleanups, hook);
        wait_for_status(&result.status, StreamStatus::Done).await;
        assert_eq!(result.chunks.get(), vec![1], "the factory ran again");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_auto_start_false_stays_idle_and_never_calls_the_factory() {
        let mut store = HookStore::new();
        let calls = Arc::new(AtomicUsize::new(0));

        let hook = {
            let calls = Arc::clone(&calls);
            move |ctx: &mut BuildContext| {
                let calls = Arc::clone(&calls);
                use_stream(
                    ctx,
                    move || {
                        let calls = Arc::clone(&calls);
                        async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            Ok(futures::stream::iter(vec![Ok(7)]))
                        }
                    },
                    StreamOptions::new().auto_start(false),
                )
            }
        };

        let (result, cleanups) = mount(&mut store, hook.clone());
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(result.status.get(), StreamStatus::Idle);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(result.chunks.get().is_empty());

        // A restart is how an opt-in stream is started.
        result.restart();
        let (result, _cleanups) = rebuild(&mut store, cleanups, hook);
        wait_for_status(&result.status, StreamStatus::Done).await;
        assert_eq!(result.chunks.get(), vec![7]);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_stop_aborts_and_reports_idle_while_keeping_the_chunks() {
        let mut store = HookStore::new();

        let (result, _cleanups) = mount(&mut store, |ctx| {
            use_stream(
                ctx,
                || async {
                    Ok(paced_stream(
                        (0..100).collect::<Vec<_>>(),
                        Duration::from_millis(10),
                    ))
                },
                StreamOptions::new(),
            )
        });

        let chunks = result.chunks.clone();
        wait_until("the first chunk", move || !chunks.get().is_empty()).await;

        result.stop();
        let at_stop = result.chunks.get();
        assert!(!at_stop.is_empty(), "stop keeps what already arrived");
        assert_eq!(result.status.get(), StreamStatus::Idle);

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(result.chunks.get(), at_stop);
        assert_eq!(result.status.get(), StreamStatus::Idle);
    }

    #[tokio::test]
    async fn test_max_chunks_keeps_only_the_most_recent() {
        let mut store = HookStore::new();

        let (result, _cleanups) = mount(&mut store, |ctx| {
            use_stream(
                ctx,
                || async { Ok(futures::stream::iter((1..=10).map(Ok))) },
                StreamOptions::new().max_chunks(3),
            )
        });

        wait_for_status(&result.status, StreamStatus::Done).await;
        assert_eq!(result.chunks.get(), vec![8, 9, 10]);
    }

    #[tokio::test]
    async fn test_use_stream_text_concatenates_its_chunks() {
        let mut store = HookStore::new();

        let (result, _cleanups) = mount(&mut store, |ctx| {
            use_stream_text(
                ctx,
                || async {
                    Ok(futures::stream::iter(vec![
                        Ok("Hello".to_string()),
                        Ok(", ".to_string()),
                        Ok("world".to_string()),
                    ]))
                },
                StreamOptions::new(),
            )
        });

        wait_for_status(&result.status, StreamStatus::Done).await;
        assert_eq!(result.text.get(), "Hello, world");
        assert!(!result.is_streaming());
        assert!(format!("{result:?}").contains("Hello, world"));
    }

    #[tokio::test]
    async fn test_use_stream_text_caps_appended_chunks() {
        let mut store = HookStore::new();

        let (result, _cleanups) = mount(&mut store, |ctx| {
            use_stream_text(
                ctx,
                || async {
                    Ok(futures::stream::iter(
                        ["a", "b", "c", "d"].map(|s| Ok(s.to_string())),
                    ))
                },
                StreamOptions::new().max_chunks(2),
            )
        });

        wait_for_status(&result.status, StreamStatus::Done).await;
        assert_eq!(result.text.get(), "ab");
    }

    #[tokio::test]
    async fn test_use_stream_text_reports_a_failure() {
        let mut store = HookStore::new();

        let (result, _cleanups) = mount(&mut store, |ctx| {
            use_stream_text(
                ctx,
                || async {
                    Ok(futures::stream::iter(vec![
                        Ok("partial".to_string()),
                        Err(QueryError::new("token stream broke")),
                    ]))
                },
                StreamOptions::new(),
            )
        });

        wait_for_status(
            &result.status,
            StreamStatus::Error("token stream broke".to_string()),
        )
        .await;
        assert_eq!(
            result.text.get(),
            "partial",
            "what arrived before the error stays"
        );
    }

    #[tokio::test]
    async fn test_use_stream_text_restart_clears_the_text() {
        let mut store = HookStore::new();
        let calls = Arc::new(AtomicUsize::new(0));

        let hook = {
            let calls = Arc::clone(&calls);
            move |ctx: &mut BuildContext| {
                let calls = Arc::clone(&calls);
                use_stream_text(
                    ctx,
                    move || {
                        let calls = Arc::clone(&calls);
                        async move {
                            let call = calls.fetch_add(1, Ordering::SeqCst);
                            Ok(futures::stream::iter(vec![Ok(format!("run-{call}"))]))
                        }
                    },
                    StreamOptions::new(),
                )
            }
        };

        let (result, cleanups) = mount(&mut store, hook.clone());
        wait_for_status(&result.status, StreamStatus::Done).await;
        assert_eq!(result.text.get(), "run-0");

        result.restart();
        assert_eq!(result.text.get(), "");

        let (result, _cleanups) = rebuild(&mut store, cleanups, hook);
        wait_for_status(&result.status, StreamStatus::Done).await;
        assert_eq!(result.text.get(), "run-1");
    }

    #[tokio::test]
    async fn test_use_stream_text_stop_keeps_the_text_and_reports_idle() {
        let mut store = HookStore::new();

        let (result, _cleanups) = mount(&mut store, |ctx| {
            use_stream_text(
                ctx,
                || async {
                    Ok(futures::stream::unfold(0usize, |seen| async move {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        Some((Ok(format!("{seen} ")), seen + 1))
                    }))
                },
                StreamOptions::new(),
            )
        });

        let text = result.text.clone();
        wait_until("the first token", move || !text.get().is_empty()).await;
        result.stop();

        let at_stop = result.text.get();
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert_eq!(result.text.get(), at_stop, "the task was aborted");
        assert_eq!(result.status.get(), StreamStatus::Idle);
    }

    #[tokio::test]
    async fn test_result_is_cloneable_and_shares_its_state() {
        let mut store = HookStore::new();

        let (result, _cleanups) = mount(&mut store, |ctx| {
            use_stream(
                ctx,
                || async { Ok(futures::stream::iter(vec![Ok(42)])) },
                StreamOptions::new(),
            )
        });
        let captured = result.clone();

        wait_for_status(&result.status, StreamStatus::Done).await;
        assert_eq!(captured.chunks.get(), vec![42]);
        assert_eq!(captured.status.get(), StreamStatus::Done);
    }

    #[test]
    fn test_default_options_auto_start_with_no_retries() {
        let options = StreamOptions::default();
        assert!(options.auto_start);
        assert_eq!(options.max_retries, 0);
        assert_eq!(options.retry_delay, Duration::from_secs(1));
        assert_eq!(options.max_chunks, None);

        let tuned = StreamOptions::new()
            .auto_start(false)
            .max_retries(3)
            .retry_delay(Duration::from_millis(250))
            .max_chunks(64);
        assert!(!tuned.auto_start);
        assert_eq!(tuned.max_retries, 3);
        assert_eq!(tuned.retry_delay, Duration::from_millis(250));
        assert_eq!(tuned.max_chunks, Some(64));
    }

    #[test]
    fn test_stream_status_defaults_to_idle() {
        assert_eq!(StreamStatus::default(), StreamStatus::Idle);
    }
}
