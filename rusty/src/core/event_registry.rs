use std::collections::HashMap;
use std::sync::Arc;

/// Typed event names for compile-time safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventName {
    Click,
    Change,
    CellClick,
    RowAction,
    Submit,
    LineClick,
    DayClick,
    Input,
    Resize,
    LinkClick,
    Focus,
    Blur,
}

impl EventName {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventName::Click => "click",
            EventName::Change => "change",
            EventName::CellClick => "cellclick",
            EventName::RowAction => "rowaction",
            EventName::Submit => "submit",
            EventName::LineClick => "lineclick",
            EventName::DayClick => "dayclick",
            EventName::Input => "input",
            EventName::Resize => "resize",
            EventName::LinkClick => "linkclick",
            EventName::Focus => "focus",
            EventName::Blur => "blur",
        }
    }

    /// Canonicalize a wire event name: strips a leading `on` and lowercases,
    /// so `onClick`, `click` and `Click` all map to `EventName::Click`.
    pub fn canonicalize(s: &str) -> String {
        let trimmed = s.strip_prefix("on").filter(|r| !r.is_empty()).unwrap_or(s);
        trimmed.to_ascii_lowercase()
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match Self::canonicalize(s).as_str() {
            "click" => Some(EventName::Click),
            "change" => Some(EventName::Change),
            "cellclick" => Some(EventName::CellClick),
            "rowaction" => Some(EventName::RowAction),
            "submit" => Some(EventName::Submit),
            "lineclick" => Some(EventName::LineClick),
            "dayclick" => Some(EventName::DayClick),
            "input" => Some(EventName::Input),
            "resize" => Some(EventName::Resize),
            "linkclick" => Some(EventName::LinkClick),
            "focus" => Some(EventName::Focus),
            "blur" => Some(EventName::Blur),
            _ => None,
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
        let canonical = EventName::from_str(event_name)
            .map(|e| e.as_str().to_string())
            .unwrap_or_else(|| event_name.to_string());
        let key = (widget_id.to_string(), canonical);
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
    fn test_event_name_round_trip() {
        let all = [
            EventName::Click,
            EventName::Change,
            EventName::CellClick,
            EventName::RowAction,
            EventName::Submit,
            EventName::LineClick,
            EventName::DayClick,
            EventName::Input,
            EventName::Resize,
            EventName::LinkClick,
            EventName::Focus,
            EventName::Blur,
        ];

        for event in all {
            assert_eq!(EventName::from_str(event.as_str()), Some(event));
            // Event names travel over the wire lowercase.
            assert_eq!(event.as_str(), event.as_str().to_lowercase());
        }

        assert_eq!(EventName::from_str("onClick"), Some(EventName::Click));
        assert_eq!(EventName::from_str("unknown"), None);
    }

    #[test]
    fn test_clear_registry() {
        let mut registry = EventRegistry::new();
        registry.register("w-0", "click", Arc::new(|_| {}));
        assert!(registry.dispatch("w-0", "click", serde_json::Value::Null));

        registry.clear();
        assert!(!registry.dispatch("w-0", "click", serde_json::Value::Null));
    }

    #[test]
    fn test_canonicalize() {
        assert_eq!(EventName::canonicalize("onClick"), "click");
        assert_eq!(EventName::canonicalize("Click"), "click");
        assert_eq!(EventName::canonicalize("onchange"), "change");
        assert_eq!(EventName::canonicalize("on"), "on"); // bare "on" left alone
        assert_eq!(EventName::canonicalize("click"), "click");
    }

    #[test]
    fn test_dispatch_with_camelcase() {
        let mut registry = EventRegistry::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        // Register handler under lowercase "click"
        registry.register(
            "w-0",
            "click",
            Arc::new(move |_args| {
                called_clone.store(true, Ordering::SeqCst);
            }),
        );

        // Dispatch with camelCase "onClick" - should find the handler
        let result = registry.dispatch("w-0", "onClick", serde_json::Value::Null);
        assert!(result);
        assert!(called.load(Ordering::SeqCst));

        // Test reverse: register with "onChange", dispatch with "change"
        let called2 = Arc::new(AtomicBool::new(false));
        let called2_clone = called2.clone();
        registry.register(
            "w-1",
            "onChange",
            Arc::new(move |_args| {
                called2_clone.store(true, Ordering::SeqCst);
            }),
        );
        let result2 = registry.dispatch("w-1", "change", serde_json::Value::Null);
        assert!(result2);
        assert!(called2.load(Ordering::SeqCst));

        // Test unknown event name with exact match
        registry.register("w-2", "customEvent", Arc::new(|_| {}));
        assert!(registry.dispatch("w-2", "customEvent", serde_json::Value::Null));
    }
}
