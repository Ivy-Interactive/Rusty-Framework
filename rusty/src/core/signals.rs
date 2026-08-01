use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use uuid::Uuid;

/// How far a signal reaches.
///
/// Ported from Ivy-Framework's `BroadcastType`. Ivy's `User` and `AppShell`
/// variants are dropped: Rusty has neither an auth service nor an app-shell
/// concept to scope them by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SignalScope {
    /// Only views in the same connection receive it.
    #[default]
    Session,
    /// Every connection on the server receives it.
    Server,
}

/// A receiver callback registered against a signal.
type Receiver<I, O> = Arc<dyn Fn(I) -> O + Send + Sync>;

/// A named channel that views send on and other views receive from.
///
/// Ported from Ivy-Framework's `Signal<TInput, TOutput>`. Cheaply cloneable; all
/// clones share the same receiver set.
pub struct Signal<I, O> {
    name: &'static str,
    receivers: Arc<Mutex<HashMap<Uuid, Receiver<I, O>>>>,
}

impl<I, O> Signal<I, O>
where
    I: Clone + Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    pub fn new(name: &'static str) -> Self {
        Signal {
            name,
            receivers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Invoke every receiver with `input` and collect their outputs.
    ///
    /// Receivers run synchronously in unspecified order (Ivy uses `Task.WhenAll`
    /// over `Task.Run`). A receiver that panics is skipped — its output is simply
    /// absent from the returned `Vec` — so one bad receiver cannot silence the
    /// others. The receiver lock is released before any receiver runs, so a
    /// receiver may itself send on the same signal.
    pub fn send(&self, input: I) -> Vec<O> {
        let receivers: Vec<Receiver<I, O>> = {
            let guard = self.receivers.lock().unwrap();
            guard.values().cloned().collect()
        };

        let mut outputs = Vec::with_capacity(receivers.len());
        for receiver in receivers {
            let input = input.clone();
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || receiver(input))) {
                Ok(output) => outputs.push(output),
                Err(_) => {
                    tracing::error!(
                        signal = self.name,
                        "a signal receiver panicked; its output is omitted"
                    );
                }
            }
        }
        outputs
    }

    /// Number of receivers currently registered.
    pub fn receiver_count(&self) -> usize {
        self.receivers.lock().unwrap().len()
    }

    /// Register a receiver. The returned handle removes it on drop.
    pub fn receive(
        &self,
        receiver_id: Uuid,
        callback: impl Fn(I) -> O + Send + Sync + 'static,
    ) -> SignalSubscription<I, O> {
        self.receivers
            .lock()
            .unwrap()
            .insert(receiver_id, Arc::new(callback));

        SignalSubscription {
            receivers: Arc::downgrade(&self.receivers),
            receiver_id,
        }
    }
}

impl<I, O> Clone for Signal<I, O> {
    fn clone(&self) -> Self {
        Signal {
            name: self.name,
            receivers: Arc::clone(&self.receivers),
        }
    }
}

impl<I, O> std::fmt::Debug for Signal<I, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signal")
            .field("name", &self.name)
            .field("receivers", &self.receivers.lock().map(|r| r.len()).ok())
            .finish()
    }
}

/// Removes its receiver on drop, replacing Ivy's `IDisposable` subscription.
pub struct SignalSubscription<I, O> {
    receivers: Weak<Mutex<HashMap<Uuid, Receiver<I, O>>>>,
    receiver_id: Uuid,
}

impl<I, O> Drop for SignalSubscription<I, O> {
    fn drop(&mut self) {
        if let Some(receivers) = self.receivers.upgrade() {
            receivers.lock().unwrap().remove(&self.receiver_id);
        }
    }
}

impl<I, O> std::fmt::Debug for SignalSubscription<I, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignalSubscription")
            .field("receiver_id", &self.receiver_id)
            .finish()
    }
}

/// A signal's identity: its name plus the `TypeId` of its `(I, O)` payload pair.
type SignalKey = (&'static str, TypeId);

/// A `Signal<I, O>` with its payload types erased, so one map can hold all of them.
type ErasedSignal = Box<dyn std::any::Any + Send + Sync>;

/// The set of signals for one scope, keyed by name **and** payload types.
///
/// Ivy keys its signal bag by the signal's .NET type; Rusty keys by
/// `(name, TypeId::of::<(I, O)>())` so two signals that happen to share a name
/// but carry different payloads cannot be confused for one another.
pub struct SignalRegistry {
    signals: Mutex<HashMap<SignalKey, ErasedSignal>>,
}

impl SignalRegistry {
    pub fn new() -> Self {
        SignalRegistry {
            signals: Mutex::new(HashMap::new()),
        }
    }

    /// Get the signal named `name` carrying `I`/`O`, creating it on first use.
    pub fn get_or_create<I, O>(&self, name: &'static str) -> Signal<I, O>
    where
        I: Clone + Send + Sync + 'static,
        O: Send + Sync + 'static,
    {
        let key = (name, TypeId::of::<(I, O)>());
        let mut signals = self.signals.lock().unwrap();
        let entry = signals
            .entry(key)
            .or_insert_with(|| Box::new(Signal::<I, O>::new(name)));

        entry
            .downcast_ref::<Signal<I, O>>()
            .expect("signal keyed by its payload TypeId must downcast to that payload's Signal")
            .clone()
    }

    /// Number of distinct signals created so far.
    pub fn len(&self) -> usize {
        self.signals.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for SignalRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// The server-wide signal registry, wrapped so it can be registered alongside the
/// per-session one.
///
/// `ServiceRegistry` is keyed by type, so two bare `SignalRegistry` entries would
/// overwrite each other; this newtype gives the server-scoped registry its own key.
#[derive(Debug, Clone)]
pub struct ServerSignals(Arc<SignalRegistry>);

impl ServerSignals {
    pub fn new(registry: Arc<SignalRegistry>) -> Self {
        ServerSignals(registry)
    }

    pub fn registry(&self) -> Arc<SignalRegistry> {
        Arc::clone(&self.0)
    }
}

impl std::fmt::Debug for SignalRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignalRegistry")
            .field("signals", &self.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_send_reaches_every_receiver_and_collects_outputs() {
        let signal = Signal::<i32, i32>::new("double");

        let _a = signal.receive(Uuid::new_v4(), |n| n * 2);
        let _b = signal.receive(Uuid::new_v4(), |n| n * 10);

        let mut outputs = signal.send(3);
        outputs.sort();
        assert_eq!(outputs, vec![6, 30]);
    }

    #[test]
    fn test_send_with_no_receivers_returns_empty() {
        let signal = Signal::<String, usize>::new("len");
        assert!(signal.send("hello".to_string()).is_empty());
        assert_eq!(signal.receiver_count(), 0);
    }

    #[test]
    fn test_dropping_subscription_removes_receiver() {
        let signal = Signal::<i32, i32>::new("s");
        let calls = Arc::new(AtomicUsize::new(0));

        let subscription = {
            let calls = calls.clone();
            signal.receive(Uuid::new_v4(), move |n| {
                calls.fetch_add(1, Ordering::SeqCst);
                n
            })
        };
        assert_eq!(signal.receiver_count(), 1);
        signal.send(1);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        drop(subscription);
        assert_eq!(signal.receiver_count(), 0);
        assert!(signal.send(2).is_empty());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "a dropped receiver must not be invoked"
        );
    }

    #[test]
    fn test_one_panicking_receiver_does_not_stop_the_others() {
        let signal = Signal::<i32, i32>::new("mixed");

        // Silence the default panic hook so the expected panic doesn't spam output.
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let _good_a = signal.receive(Uuid::new_v4(), |n| n + 1);
        let _bad = signal.receive(Uuid::new_v4(), |_| panic!("receiver blew up"));
        let _good_b = signal.receive(Uuid::new_v4(), |n| n + 2);

        let mut outputs = signal.send(10);
        std::panic::set_hook(previous_hook);

        outputs.sort();
        assert_eq!(
            outputs,
            vec![11, 12],
            "the surviving receivers' outputs are still collected"
        );
    }

    #[test]
    fn test_registry_returns_the_same_signal_for_one_name() {
        let registry = SignalRegistry::new();

        let a = registry.get_or_create::<i32, i32>("refresh");
        let b = registry.get_or_create::<i32, i32>("refresh");

        let _sub = a.receive(Uuid::new_v4(), |n| n);
        assert_eq!(
            b.receiver_count(),
            1,
            "both handles must share one receiver set"
        );
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_keys_by_payload_type_as_well_as_name() {
        let registry = SignalRegistry::new();

        let ints = registry.get_or_create::<i32, i32>("go");
        let strings = registry.get_or_create::<String, usize>("go");

        let _int_sub = ints.receive(Uuid::new_v4(), |n| n);
        assert_eq!(
            strings.receiver_count(),
            0,
            "same name, different payload types => distinct signals"
        );
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_clones_of_a_signal_share_receivers() {
        let signal = Signal::<i32, i32>::new("shared");
        let clone = signal.clone();

        let _sub = clone.receive(Uuid::new_v4(), |n| n * 3);
        assert_eq!(signal.send(5), vec![15]);
    }

    #[test]
    fn test_receiver_may_send_on_the_same_signal() {
        // The receiver lock must be released before receivers run, or this deadlocks.
        let signal = Signal::<i32, i32>::new("reentrant");
        let inner = Signal::<i32, i32>::new("inner");
        let _inner_sub = inner.receive(Uuid::new_v4(), |n| n * 2);

        let outer_signal = signal.clone();
        let _sub = signal.receive(Uuid::new_v4(), move |n| {
            // Re-enter `send` on a signal from inside a receiver.
            let _ = outer_signal.receiver_count();
            n
        });

        assert_eq!(signal.send(7), vec![7]);
        assert_eq!(inner.send(7), vec![14]);
    }
}
