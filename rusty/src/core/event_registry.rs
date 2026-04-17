use std::collections::HashMap;
use std::sync::Arc;

/// Typed event names for compile-time safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventName {
    Click,
    Change,
}

impl EventName {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventName::Click => "click",
            EventName::Change => "change",
        }
    }
}

impl std::str::FromStr for EventName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "click" => Ok(EventName::Click),
            "change" => Ok(EventName::Change),
            _ => Err(format!("unknown event name: '{}'", s)),
        }
    }
}

impl std::fmt::Display for EventName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

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

    /// Merge another registry's handlers into this one.
    pub fn merge(&mut self, other: EventRegistry) {
        self.handlers.extend(other.handlers);
    }

    /// Register a callback using a typed event name.
    pub fn register_typed(&mut self, widget_id: &str, event: EventName, callback: EventCallback) {
        self.register(widget_id, event.as_str(), callback);
    }

    /// Dispatch an event using a typed event name.
    /// Returns true if a handler was found and invoked, false otherwise.
    pub fn dispatch_typed(
        &self,
        widget_id: &str,
        event: EventName,
        args: serde_json::Value,
    ) -> bool {
        self.dispatch(widget_id, event.as_str(), args)
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
    fn test_from_str_valid() {
        assert_eq!("click".parse::<EventName>(), Ok(EventName::Click));
        assert_eq!("change".parse::<EventName>(), Ok(EventName::Change));
    }

    #[test]
    fn test_from_str_invalid() {
        assert!("unknown".parse::<EventName>().is_err());
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
