use crate::core::event_registry::EventRegistry;
use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// A rich text editor producing HTML.
///
/// Ported from Ivy's Tiptap input but named for the behaviour rather than the JS
/// library, matching Rusty's library-agnostic widget names.
#[derive(Clone, Serialize, Deserialize)]
pub struct RichTextInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    pub disabled: bool,
    pub editable: bool,
    pub auto_focus: bool,
    pub show_toolbar: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid: Option<String>,
    #[serde(skip)]
    pub on_change: Option<Arc<dyn Fn(String) + Send + Sync>>,
    #[serde(skip)]
    pub on_focus: Option<Arc<dyn Fn() + Send + Sync>>,
    #[serde(skip)]
    pub on_blur: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for RichTextInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RichTextInput")
            .field("value", &self.value)
            .field("editable", &self.editable)
            .finish()
    }
}

impl RichTextInput {
    pub fn new() -> Self {
        RichTextInput {
            id: None,
            value: None,
            placeholder: None,
            disabled: false,
            editable: true,
            auto_focus: false,
            show_toolbar: true,
            invalid: None,
            on_change: None,
            on_focus: None,
            on_blur: None,
        }
    }

    pub fn value(mut self, value: &str) -> Self {
        self.value = Some(value.to_string());
        self
    }

    pub fn placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = Some(placeholder.to_string());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Render the current value without allowing edits.
    pub fn read_only(mut self) -> Self {
        self.editable = false;
        self
    }

    pub fn auto_focus(mut self, auto_focus: bool) -> Self {
        self.auto_focus = auto_focus;
        self
    }

    pub fn show_toolbar(mut self, show: bool) -> Self {
        self.show_toolbar = show;
        self
    }

    pub fn hide_toolbar(mut self) -> Self {
        self.show_toolbar = false;
        self
    }

    /// Mark the input invalid with a validation message.
    pub fn invalid(mut self, message: &str) -> Self {
        self.invalid = Some(message.to_string());
        self
    }

    pub fn on_change(mut self, handler: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(handler));
        self
    }

    pub fn on_focus(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_focus = Some(Arc::new(handler));
        self
    }

    pub fn on_blur(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_blur = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for RichTextInput {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetData for RichTextInput {
    fn widget_type(&self) -> &str {
        "rich_text_input"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "rich_text_input",
            "id": self.id,
            "value": self.value,
            "placeholder": self.placeholder,
            "disabled": self.disabled,
            "editable": self.editable,
            "autoFocus": self.auto_focus,
            "showToolbar": self.show_toolbar,
            "invalid": self.invalid,
            "hasOnChange": self.on_change.is_some(),
            "hasOnFocus": self.on_focus.is_some(),
            "hasOnBlur": self.on_blur.is_some(),
        })
    }

    fn clone_box(&self) -> Box<dyn WidgetData> {
        Box::new(self.clone())
    }

    fn assign_id(&mut self, id: String) {
        self.id = Some(id);
    }

    fn get_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    fn register_events(&self, widget_id: &str, registry: &mut EventRegistry) {
        if let Some(handler) = &self.on_change {
            let handler = handler.clone();
            registry.register(
                widget_id,
                "change",
                Arc::new(move |args| {
                    if let Some(value) = args.get("value").and_then(|v| v.as_str()) {
                        handler(value.to_string());
                    }
                }),
            );
        }
        if let Some(handler) = &self.on_focus {
            let handler = handler.clone();
            registry.register(widget_id, "focus", Arc::new(move |_| handler()));
        }
        if let Some(handler) = &self.on_blur {
            let handler = handler.clone();
            registry.register(widget_id, "blur", Arc::new(move |_| handler()));
        }
    }
}

impl From<RichTextInput> for Element {
    fn from(input: RichTextInput) -> Self {
        input.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::BuildContext;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    #[test]
    fn test_rich_text_input_builder_round_trip() {
        let input = RichTextInput::new()
            .value("<p>Hello</p>")
            .placeholder("Write something…")
            .disabled(true)
            .auto_focus(true)
            .show_toolbar(false)
            .invalid("Too short");

        assert_eq!(input.value.as_deref(), Some("<p>Hello</p>"));
        assert_eq!(input.placeholder.as_deref(), Some("Write something…"));
        assert!(input.disabled);
        assert!(input.auto_focus);
        assert!(!input.show_toolbar);
        assert_eq!(input.invalid.as_deref(), Some("Too short"));
    }

    #[test]
    fn test_rich_text_input_defaults() {
        let input = RichTextInput::default();
        assert!(input.value.is_none());
        assert!(!input.disabled);
        assert!(input.editable);
        assert!(!input.auto_focus);
        assert!(input.show_toolbar);
        assert!(input.invalid.is_none());
    }

    #[test]
    fn test_rich_text_input_read_only_and_hide_toolbar() {
        let input = RichTextInput::new().read_only().hide_toolbar();
        assert!(!input.editable);
        assert!(!input.show_toolbar);
    }

    #[test]
    fn test_rich_text_input_to_json_keys() {
        let json = RichTextInput::new()
            .value("<p>Hi</p>")
            .on_change(|_| {})
            .on_focus(|| {})
            .on_blur(|| {})
            .to_json();

        assert_eq!(json["type"], "rich_text_input");
        assert_eq!(json["value"], "<p>Hi</p>");
        assert_eq!(json["editable"], true);
        assert_eq!(json["showToolbar"], true);
        assert_eq!(json["hasOnChange"], true);
        assert_eq!(json["hasOnFocus"], true);
        assert_eq!(json["hasOnBlur"], true);
    }

    #[test]
    fn test_rich_text_input_json_without_handlers() {
        let json = RichTextInput::new().to_json();
        assert_eq!(json["hasOnChange"], false);
        assert_eq!(json["hasOnFocus"], false);
        assert_eq!(json["hasOnBlur"], false);
    }

    #[test]
    fn test_rich_text_input_assign_ids() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let mut element: Element = RichTextInput::new().into();
        element.assign_ids(&mut ctx);
        if let Element::Widget(ref w) = element {
            assert_eq!(w.get_id(), Some("w-0"));
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_rich_text_input_change_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received = Arc::new(Mutex::new(None::<String>));
        let received_clone = received.clone();
        let mut element: Element = RichTextInput::new()
            .on_change(move |value| {
                *received_clone.lock().unwrap() = Some(value);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "change", json!({"value": "<p>Edited</p>"})));
        assert_eq!(received.lock().unwrap().as_deref(), Some("<p>Edited</p>"));
    }

    #[test]
    fn test_rich_text_input_focus_and_blur_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let focus_count = Arc::new(AtomicUsize::new(0));
        let blur_count = Arc::new(AtomicUsize::new(0));
        let focus_clone = focus_count.clone();
        let blur_clone = blur_count.clone();

        let mut element: Element = RichTextInput::new()
            .on_focus(move || {
                focus_clone.fetch_add(1, Ordering::SeqCst);
            })
            .on_blur(move || {
                blur_clone.fetch_add(1, Ordering::SeqCst);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "focus", serde_json::Value::Null));
        assert!(registry.dispatch("w-0", "blur", serde_json::Value::Null));
        assert_eq!(focus_count.load(Ordering::SeqCst), 1);
        assert_eq!(blur_count.load(Ordering::SeqCst), 1);
    }
}
