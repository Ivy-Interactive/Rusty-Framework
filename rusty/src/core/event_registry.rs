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
    Toggle,
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
            EventName::Toggle => "toggle",
        }
    }

    /// Parse a wire event name. Accepts both the canonical lowercase form
    /// (`"click"`) and the camelCase form the browser sends (`"onClick"`).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match Self::normalize(s).as_str() {
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
            "toggle" => Some(EventName::Toggle),
            _ => None,
        }
    }

    /// Strip a leading `on` handler prefix and lowercase the rest, so
    /// `onCellClick`, `cellClick` and `cellclick` all normalize alike.
    fn normalize(s: &str) -> String {
        s.strip_prefix("on")
            .filter(|rest| rest.starts_with(|c: char| c.is_ascii_uppercase()))
            .unwrap_or(s)
            .to_ascii_lowercase()
    }

    /// Resolve a wire event name to its canonical string, leaving unrecognized
    /// names (custom `#[event]` fields) untouched.
    pub fn canonicalize(s: &str) -> &str {
        match Self::from_str(s) {
            Some(event) => event.as_str(),
            None => s,
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
    ///
    /// The name is stored canonically, so registering `click` and `onClick`
    /// both land under the same key that [`EventRegistry::dispatch`] looks up.
    pub fn register(&mut self, widget_id: &str, event_name: &str, callback: EventCallback) {
        let canonical = EventName::canonicalize(event_name);
        self.handlers
            .insert((widget_id.to_string(), canonical.to_string()), callback);
    }

    /// Dispatch an event to the registered handler.
    ///
    /// The incoming name is resolved through [`EventName::canonicalize`], so a
    /// browser sending `onClick` reaches a handler registered as `click`.
    /// Unrecognized names fall back to the raw string, keeping custom
    /// `#[event]` fields working.
    /// Returns true if a handler was found and invoked, false otherwise.
    pub fn dispatch(&self, widget_id: &str, event_name: &str, args: serde_json::Value) -> bool {
        let canonical = EventName::canonicalize(event_name);
        let key = (widget_id.to_string(), canonical.to_string());
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
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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
            EventName::Toggle,
        ];

        for event in all {
            assert_eq!(EventName::from_str(event.as_str()), Some(event));
            // Event names travel over the wire lowercase.
            assert_eq!(event.as_str(), event.as_str().to_lowercase());
        }

        assert_eq!(EventName::from_str("unknown"), None);
    }

    #[test]
    fn test_from_str_accepts_camel_case_wire_names() {
        // The E2E renderer sends camelCase handler names; both forms must parse.
        assert_eq!(EventName::from_str("onClick"), Some(EventName::Click));
        assert_eq!(EventName::from_str("onChange"), Some(EventName::Change));
        assert_eq!(EventName::from_str("onToggle"), Some(EventName::Toggle));
        assert_eq!(
            EventName::from_str("onCellClick"),
            Some(EventName::CellClick)
        );
        assert_eq!(EventName::from_str("cellClick"), Some(EventName::CellClick));

        // A leading `on` that is not a handler prefix must not be stripped.
        assert_eq!(EventName::from_str("online"), None);
    }

    #[test]
    fn test_canonicalize_falls_back_to_raw_name() {
        assert_eq!(EventName::canonicalize("onClick"), "click");
        assert_eq!(EventName::canonicalize("click"), "click");
        // Custom `#[event]` field names pass through untouched.
        assert_eq!(EventName::canonicalize("myCustomEvent"), "myCustomEvent");
    }

    #[test]
    fn test_dispatch_resolves_camel_case_to_lowercase_registration() {
        let mut registry = EventRegistry::new();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_clone = hits.clone();

        registry.register(
            "w-0",
            "click",
            Arc::new(move |_| {
                hits_clone.fetch_add(1, Ordering::SeqCst);
            }),
        );

        assert!(registry.dispatch("w-0", "onClick", serde_json::Value::Null));
        assert!(registry.dispatch("w-0", "click", serde_json::Value::Null));
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_dispatch_resolves_lowercase_to_camel_case_registration() {
        let mut registry = EventRegistry::new();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_clone = hits.clone();

        // The reverse direction: registered camelCase, dispatched lowercase.
        registry.register(
            "w-0",
            "onChange",
            Arc::new(move |_| {
                hits_clone.fetch_add(1, Ordering::SeqCst);
            }),
        );

        assert!(registry.dispatch("w-0", "change", serde_json::Value::Null));
        assert!(registry.dispatch("w-0", "onChange", serde_json::Value::Null));
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_dispatch_unrecognized_name_uses_raw_string() {
        let mut registry = EventRegistry::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        registry.register(
            "w-0",
            "customThing",
            Arc::new(move |_| {
                called_clone.store(true, Ordering::SeqCst);
            }),
        );

        assert!(registry.dispatch("w-0", "customThing", serde_json::Value::Null));
        assert!(called.load(Ordering::SeqCst));
        // Not a known EventName, so no casing normalization is applied.
        assert!(!registry.dispatch("w-0", "customthing", serde_json::Value::Null));
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
