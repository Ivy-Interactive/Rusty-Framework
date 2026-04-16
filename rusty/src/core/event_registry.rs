use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for event handler callbacks.
/// Receives the event arguments as a serde_json::Value.
pub type EventCallback = Arc<dyn Fn(serde_json::Value) + Send + Sync>;

/// Thread-safe registry mapping (widget_id, event_name) to callback closures.
/// Populated during tree construction and queried during event dispatch.
pub struct EventRegistry {
    handlers: HashMap<(String, String), EventCallback>,
}

impl EventRegistry {
    pub fn new() -> Self {
        EventRegistry {
            handlers: HashMap::new(),
        }
    }

    /// Register a callback for a specific widget and event.
    pub fn register(&mut self, widget_id: &str, event_name: &str, callback: EventCallback) {
        self.handlers
            .insert((widget_id.to_string(), event_name.to_string()), callback);
    }

    /// Dispatch an event to the registered handler.
    /// Returns true if a handler was found and invoked, false otherwise.
    pub fn dispatch(&self, widget_id: &str, event_name: &str, args: serde_json::Value) -> bool {
        let key = (widget_id.to_string(), event_name.to_string());
        if let Some(handler) = self.handlers.get(&key) {
            handler(args);
            true
        } else {
            false
        }
    }

    /// Remove all registered handlers.
    pub fn clear(&mut self) {
        self.handlers.clear();
    }
}

impl Default for EventRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_register_and_dispatch() {
        let mut registry = EventRegistry::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        registry.register(
            "w-0",
            "click",
            Arc::new(move |_args| {
                called_clone.store(true, Ordering::SeqCst);
            }),
        );

        let result = registry.dispatch("w-0", "click", serde_json::Value::Null);
        assert!(result);
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_dispatch_unknown_widget() {
        let registry = EventRegistry::new();
        let result = registry.dispatch("nonexistent", "click", serde_json::Value::Null);
        assert!(!result);
    }

    #[test]
    fn test_clear_registry() {
        let mut registry = EventRegistry::new();
        registry.register("w-0", "click", Arc::new(|_| {}));
        assert!(registry.dispatch("w-0", "click", serde_json::Value::Null));

        registry.clear();
        assert!(!registry.dispatch("w-0", "click", serde_json::Value::Null));
    }
}
