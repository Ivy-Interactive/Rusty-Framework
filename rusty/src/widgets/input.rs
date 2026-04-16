use crate::core::event_registry::EventRegistry;
use crate::views::view::{BuildContext, Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// A text input widget.
#[derive(Clone, Serialize, Deserialize)]
pub struct TextInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub disabled: bool,
    pub read_only: bool,
    #[serde(skip)]
    pub on_change: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl std::fmt::Debug for TextInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextInput")
            .field("value", &self.value)
            .field("label", &self.label)
            .finish()
    }
}

impl TextInput {
    pub fn new() -> Self {
        TextInput {
            id: None,
            value: None,
            placeholder: None,
            label: None,
            disabled: false,
            read_only: false,
            on_change: None,
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

    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(handler));
        self
    }

    /// Assign a widget ID from the BuildContext and register event handlers.
    #[deprecated(note = "Widget IDs are now assigned automatically. Remove .build(ctx) calls.")]
    pub fn build(mut self, ctx: &mut BuildContext) -> Self {
        let widget_id = ctx.next_widget_id();
        self.id = Some(widget_id.clone());
        if let Some(handler) = &self.on_change {
            let handler = handler.clone();
            ctx.register_event(
                &widget_id,
                "change",
                Arc::new(move |args| {
                    if let Some(value) = args.get("value").and_then(|v| v.as_str()) {
                        handler(value.to_string());
                    }
                }),
            );
        }
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetData for TextInput {
    fn widget_type(&self) -> &str {
        "text_input"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "text_input",
            "id": self.id,
            "value": self.value,
            "placeholder": self.placeholder,
            "label": self.label,
            "disabled": self.disabled,
            "readOnly": self.read_only,
            "hasOnChange": self.on_change.is_some(),
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
    }
}

impl From<TextInput> for Element {
    fn from(input: TextInput) -> Self {
        input.into_element()
    }
}

/// A numeric input widget.
#[derive(Clone, Serialize, Deserialize)]
pub struct NumberInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub disabled: bool,
    #[serde(skip)]
    pub on_change: Option<Arc<dyn Fn(f64) + Send + Sync>>,
}

impl std::fmt::Debug for NumberInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NumberInput")
            .field("value", &self.value)
            .field("label", &self.label)
            .finish()
    }
}

impl NumberInput {
    pub fn new() -> Self {
        NumberInput {
            id: None,
            value: None,
            min: None,
            max: None,
            step: None,
            label: None,
            disabled: false,
            on_change: None,
        }
    }

    pub fn value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self
    }

    pub fn min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    pub fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(handler));
        self
    }

    /// Assign a widget ID from the BuildContext and register event handlers.
    #[deprecated(note = "Widget IDs are now assigned automatically. Remove .build(ctx) calls.")]
    pub fn build(mut self, ctx: &mut BuildContext) -> Self {
        let widget_id = ctx.next_widget_id();
        self.id = Some(widget_id.clone());
        if let Some(handler) = &self.on_change {
            let handler = handler.clone();
            ctx.register_event(
                &widget_id,
                "change",
                Arc::new(move |args| {
                    if let Some(value) = args.get("value").and_then(|v| v.as_f64()) {
                        handler(value);
                    }
                }),
            );
        }
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for NumberInput {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetData for NumberInput {
    fn widget_type(&self) -> &str {
        "number_input"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "number_input",
            "id": self.id,
            "value": self.value,
            "min": self.min,
            "max": self.max,
            "step": self.step,
            "label": self.label,
            "disabled": self.disabled,
            "hasOnChange": self.on_change.is_some(),
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
                    if let Some(value) = args.get("value").and_then(|v| v.as_f64()) {
                        handler(value);
                    }
                }),
            );
        }
    }
}

impl From<NumberInput> for Element {
    fn from(input: NumberInput) -> Self {
        input.into_element()
    }
}

/// A dropdown select widget.
#[derive(Clone, Serialize, Deserialize)]
pub struct Select {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub options: Vec<SelectOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    pub disabled: bool,
    #[serde(skip)]
    pub on_change: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl std::fmt::Debug for Select {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Select")
            .field("options", &self.options)
            .field("value", &self.value)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

impl Select {
    pub fn new(options: Vec<SelectOption>) -> Self {
        Select {
            id: None,
            options,
            value: None,
            label: None,
            placeholder: None,
            disabled: false,
            on_change: None,
        }
    }

    pub fn value(mut self, value: &str) -> Self {
        self.value = Some(value.to_string());
        self
    }

    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = Some(placeholder.to_string());
        self
    }

    pub fn on_change(mut self, handler: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(handler));
        self
    }

    /// Assign a widget ID from the BuildContext and register event handlers.
    #[deprecated(note = "Widget IDs are now assigned automatically. Remove .build(ctx) calls.")]
    pub fn build(mut self, ctx: &mut BuildContext) -> Self {
        let widget_id = ctx.next_widget_id();
        self.id = Some(widget_id.clone());
        if let Some(handler) = &self.on_change {
            let handler = handler.clone();
            ctx.register_event(
                &widget_id,
                "change",
                Arc::new(move |args| {
                    if let Some(value) = args.get("value").and_then(|v| v.as_str()) {
                        handler(value.to_string());
                    }
                }),
            );
        }
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for Select {
    fn widget_type(&self) -> &str {
        "select"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "select",
            "id": self.id,
            "options": self.options,
            "value": self.value,
            "label": self.label,
            "placeholder": self.placeholder,
            "disabled": self.disabled,
            "hasOnChange": self.on_change.is_some(),
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
    }
}

impl From<Select> for Element {
    fn from(select: Select) -> Self {
        select.into_element()
    }
}

/// A checkbox widget.
#[derive(Clone, Serialize, Deserialize)]
pub struct Checkbox {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub checked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub disabled: bool,
    #[serde(skip)]
    pub on_change: Option<Arc<dyn Fn(bool) + Send + Sync>>,
}

impl std::fmt::Debug for Checkbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checkbox")
            .field("checked", &self.checked)
            .field("label", &self.label)
            .finish()
    }
}

impl Checkbox {
    pub fn new(checked: bool) -> Self {
        Checkbox {
            id: None,
            checked,
            label: None,
            disabled: false,
            on_change: None,
        }
    }

    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(handler));
        self
    }

    /// Assign a widget ID from the BuildContext and register event handlers.
    #[deprecated(note = "Widget IDs are now assigned automatically. Remove .build(ctx) calls.")]
    pub fn build(mut self, ctx: &mut BuildContext) -> Self {
        let widget_id = ctx.next_widget_id();
        self.id = Some(widget_id.clone());
        if let Some(handler) = &self.on_change {
            let handler = handler.clone();
            ctx.register_event(
                &widget_id,
                "change",
                Arc::new(move |args| {
                    if let Some(value) = args.get("value").and_then(|v| v.as_bool()) {
                        handler(value);
                    }
                }),
            );
        }
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for Checkbox {
    fn widget_type(&self) -> &str {
        "checkbox"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "checkbox",
            "id": self.id,
            "checked": self.checked,
            "label": self.label,
            "disabled": self.disabled,
            "hasOnChange": self.on_change.is_some(),
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
                    if let Some(value) = args.get("value").and_then(|v| v.as_bool()) {
                        handler(value);
                    }
                }),
            );
        }
    }
}

impl From<Checkbox> for Element {
    fn from(checkbox: Checkbox) -> Self {
        checkbox.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;

    #[test]
    fn test_text_input_builder() {
        let input = TextInput::new().placeholder("Enter text").label("Name");
        assert_eq!(input.placeholder.as_deref(), Some("Enter text"));
        assert_eq!(input.label.as_deref(), Some("Name"));
    }

    #[test]
    fn test_number_input_range() {
        let input = NumberInput::new().min(0.0).max(100.0).step(5.0);
        assert_eq!(input.min, Some(0.0));
        assert_eq!(input.max, Some(100.0));
        assert_eq!(input.step, Some(5.0));
    }

    #[test]
    fn test_select_builder() {
        let opts = vec![
            SelectOption {
                value: "a".into(),
                label: "Alpha".into(),
            },
            SelectOption {
                value: "b".into(),
                label: "Beta".into(),
            },
        ];
        let select = Select::new(opts).value("a");
        assert_eq!(select.options.len(), 2);
        assert_eq!(select.value.as_deref(), Some("a"));
    }

    #[test]
    fn test_checkbox() {
        let cb = Checkbox::new(true).label("Accept terms");
        assert!(cb.checked);
        assert_eq!(cb.label.as_deref(), Some("Accept terms"));
    }

    #[test]
    fn test_text_input_json_includes_id() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let input = TextInput::new().build(&mut ctx);
        let json = input.to_json();
        assert_eq!(json["id"], "w-0");
        assert_eq!(json["type"], "text_input");
    }

    #[test]
    fn test_number_input_json_includes_id() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let input = NumberInput::new().build(&mut ctx);
        let json = input.to_json();
        assert_eq!(json["id"], "w-0");
    }

    #[test]
    fn test_select_json_includes_id() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let select = Select::new(vec![]).build(&mut ctx);
        let json = select.to_json();
        assert_eq!(json["id"], "w-0");
    }

    #[test]
    fn test_checkbox_json_includes_id() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let cb = Checkbox::new(false).build(&mut ctx);
        let json = cb.to_json();
        assert_eq!(json["id"], "w-0");
    }

    #[test]
    fn test_widget_ids_are_sequential() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let input1 = TextInput::new().build(&mut ctx);
        let input2 = TextInput::new().build(&mut ctx);
        assert_eq!(input1.id, Some("w-0".to_string()));
        assert_eq!(input2.id, Some("w-1".to_string()));
    }
}
