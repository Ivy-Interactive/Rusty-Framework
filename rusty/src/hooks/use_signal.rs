use std::sync::Arc;

use uuid::Uuid;

use crate::core::signals::{ServerSignals, Signal, SignalRegistry, SignalScope};
use crate::hooks::use_ref::use_ref;
use crate::views::view::BuildContext;

/// Get the named signal for this scope, so a view can send on it or receive from it.
///
/// Ported from Ivy-Framework's `UseSignal.cs`. The signal is identified by `name`
/// plus its payload types, so two unrelated signals cannot collide. A view
/// receiving from a signal should hold the [`SignalSubscription`] in a `use_ref` —
/// dropping it unregisters the receiver.
///
/// [`SignalScope::Session`] resolves the per-connection registry;
/// [`SignalScope::Server`] resolves the server-wide one.
///
/// [`SignalSubscription`]: crate::core::signals::SignalSubscription
pub fn use_signal<I, O>(
    ctx: &mut BuildContext,
    name: &'static str,
    scope: SignalScope,
) -> Signal<I, O>
where
    I: Clone + Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    // A stable receiver id per call site, so re-registering on rebuild replaces
    // this view's receiver instead of adding another one.
    let _receiver_id = use_receiver_id(ctx);

    let registry = match scope {
        SignalScope::Session => ctx
            .services()
            .get::<SignalRegistry>()
            .expect("use_signal requires a SignalRegistry on the ServiceRegistry"),
        SignalScope::Server => ctx
            .services()
            .get::<ServerSignals>()
            .expect("SignalScope::Server requires a ServerSignals on the ServiceRegistry")
            .registry(),
    };

    registry.get_or_create::<I, O>(name)
}

/// A stable per-call-site receiver id, for views that register a receiver.
///
/// Call this right after `use_signal` and pass the id to `Signal::receive`, so a
/// rebuild replaces the previous receiver rather than stacking a new one.
pub fn use_receiver_id(ctx: &mut BuildContext) -> Uuid {
    use_ref(ctx, Uuid::new_v4()).get()
}

/// Resolve the signal registry for `scope` without consuming a hook slot.
///
/// Useful for code that needs the registry itself (a session builder, say) rather
/// than one signal.
pub fn signal_registry(ctx: &BuildContext, scope: SignalScope) -> Arc<SignalRegistry> {
    match scope {
        SignalScope::Session => ctx
            .services()
            .get::<SignalRegistry>()
            .expect("SignalScope::Session requires a SignalRegistry on the ServiceRegistry"),
        SignalScope::Server => ctx
            .services()
            .get::<ServerSignals>()
            .expect("SignalScope::Server requires a ServerSignals on the ServiceRegistry")
            .registry(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::services::ServiceRegistry;
    use crate::hooks::hook_store::HookStore;
    use std::sync::Mutex;

    /// A registry with both a session and a server signal registry, as a session has.
    fn test_services() -> (
        Arc<ServiceRegistry>,
        Arc<SignalRegistry>,
        Arc<SignalRegistry>,
    ) {
        let session_signals = Arc::new(SignalRegistry::new());
        let server_signals = Arc::new(SignalRegistry::new());
        let services = Arc::new(ServiceRegistry::new());
        services.register(Arc::clone(&session_signals));
        services.register(Arc::new(ServerSignals::new(Arc::clone(&server_signals))));
        (services, session_signals, server_signals)
    }

    #[test]
    fn test_two_views_in_one_session_share_a_signal() {
        let (services, _session, _server) = test_services();

        let mut sender_store = HookStore::new();
        let mut receiver_store = HookStore::new();

        // The receiving view registers a receiver and keeps the subscription alive.
        let received = Arc::new(Mutex::new(Vec::<String>::new()));
        let subscription = {
            let mut ctx =
                BuildContext::new(&mut receiver_store, None).using_services(Arc::clone(&services));
            let signal = use_signal::<String, ()>(&mut ctx, "refresh", SignalScope::Session);
            let receiver_id = use_receiver_id(&mut ctx);
            let received = received.clone();
            signal.receive(receiver_id, move |msg| {
                received.lock().unwrap().push(msg);
            })
        };

        // The sending view gets the same signal by name.
        {
            let mut ctx =
                BuildContext::new(&mut sender_store, None).using_services(Arc::clone(&services));
            let signal = use_signal::<String, ()>(&mut ctx, "refresh", SignalScope::Session);
            signal.send("go".to_string());
        }

        assert_eq!(*received.lock().unwrap(), vec!["go".to_string()]);
        drop(subscription);
    }

    #[test]
    fn test_session_and_server_scopes_are_separate_signals() {
        let (services, session_signals, server_signals) = test_services();
        let mut store = HookStore::new();

        {
            let mut ctx = BuildContext::new(&mut store, None).using_services(Arc::clone(&services));
            let _session_signal = use_signal::<i32, i32>(&mut ctx, "ping", SignalScope::Session);
            let _server_signal = use_signal::<i32, i32>(&mut ctx, "ping", SignalScope::Server);
        }

        assert_eq!(session_signals.len(), 1);
        assert_eq!(server_signals.len(), 1);

        // Registering on one scope leaves the other untouched.
        let session_signal = session_signals.get_or_create::<i32, i32>("ping");
        let server_signal = server_signals.get_or_create::<i32, i32>("ping");
        let _sub = session_signal.receive(Uuid::new_v4(), |n| n);
        assert_eq!(session_signal.receiver_count(), 1);
        assert_eq!(
            server_signal.receiver_count(),
            0,
            "Session and Server scopes must not share receivers"
        );
    }

    #[test]
    fn test_two_sessions_do_not_share_session_scoped_signals() {
        let (services_a, _session_a, server) = test_services();

        // A second session with its own session registry but the same server one.
        let session_b = Arc::new(SignalRegistry::new());
        let services_b = Arc::new(ServiceRegistry::new());
        services_b.register(Arc::clone(&session_b));
        services_b.register(Arc::new(ServerSignals::new(Arc::clone(&server))));

        let received_a = Arc::new(Mutex::new(0usize));
        let mut store_a = HookStore::new();
        let _sub_a = {
            let mut ctx =
                BuildContext::new(&mut store_a, None).using_services(Arc::clone(&services_a));
            let signal = use_signal::<i32, ()>(&mut ctx, "tick", SignalScope::Session);
            let receiver_id = use_receiver_id(&mut ctx);
            let received_a = received_a.clone();
            signal.receive(receiver_id, move |_| {
                *received_a.lock().unwrap() += 1;
            })
        };

        // Session B sends on its own "tick"; session A must not hear it.
        let mut store_b = HookStore::new();
        {
            let mut ctx =
                BuildContext::new(&mut store_b, None).using_services(Arc::clone(&services_b));
            let signal = use_signal::<i32, ()>(&mut ctx, "tick", SignalScope::Session);
            signal.send(1);
        }
        assert_eq!(*received_a.lock().unwrap(), 0);

        // A server-scoped signal does reach both.
        let received_server = Arc::new(Mutex::new(0usize));
        let _sub_server = {
            let mut ctx =
                BuildContext::new(&mut store_a, None).using_services(Arc::clone(&services_a));
            let signal = use_signal::<i32, ()>(&mut ctx, "broadcast", SignalScope::Server);
            let received_server = received_server.clone();
            signal.receive(Uuid::new_v4(), move |_| {
                *received_server.lock().unwrap() += 1;
            })
        };
        {
            let mut ctx = BuildContext::new(&mut store_b, None).using_services(services_b);
            let signal = use_signal::<i32, ()>(&mut ctx, "broadcast", SignalScope::Server);
            signal.send(1);
        }
        assert_eq!(*received_server.lock().unwrap(), 1);
    }

    #[test]
    fn test_receiver_id_is_stable_across_rebuilds() {
        let (services, _session, _server) = test_services();
        let mut store = HookStore::new();

        let first = {
            let mut ctx = BuildContext::new(&mut store, None).using_services(Arc::clone(&services));
            let _signal = use_signal::<i32, i32>(&mut ctx, "s", SignalScope::Session);
            use_receiver_id(&mut ctx)
        };
        let second = {
            let mut ctx = BuildContext::new(&mut store, None).using_services(services);
            let _signal = use_signal::<i32, i32>(&mut ctx, "s", SignalScope::Session);
            use_receiver_id(&mut ctx)
        };

        assert_eq!(
            first, second,
            "a rebuild must reuse the receiver id so re-registering replaces the receiver"
        );
    }

    #[test]
    #[should_panic(expected = "use_signal requires a SignalRegistry")]
    fn test_missing_registry_panics() {
        let mut store = HookStore::new();
        let mut ctx = BuildContext::new(&mut store, None);
        let _ = use_signal::<i32, i32>(&mut ctx, "s", SignalScope::Session);
    }
}
