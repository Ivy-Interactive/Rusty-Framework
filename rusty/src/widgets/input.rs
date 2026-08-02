use crate::core::event_registry::EventRegistry;
use crate::views::view::{BuildContext, Element, WidgetData};
use crate::widgets::separator::Orientation;
use rusty_macros::Widget;
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

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
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

/// A multi-line text input.
#[derive(Clone, Serialize, Deserialize)]
pub struct TextArea {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<usize>,
    pub disabled: bool,
    #[serde(skip)]
    pub on_change: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl std::fmt::Debug for TextArea {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextArea")
            .field("value", &self.value)
            .field("label", &self.label)
            .finish()
    }
}

impl TextArea {
    pub fn new() -> Self {
        TextArea {
            id: None,
            value: None,
            placeholder: None,
            label: None,
            rows: None,
            disabled: false,
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

    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = Some(rows);
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

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for TextArea {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetData for TextArea {
    fn widget_type(&self) -> &str {
        "text_area"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "text_area",
            "id": self.id,
            "value": self.value,
            "placeholder": self.placeholder,
            "label": self.label,
            "rows": self.rows,
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

impl From<TextArea> for Element {
    fn from(text_area: TextArea) -> Self {
        text_area.into_element()
    }
}

/// A numeric slider bounded by `min` and `max`.
#[derive(Clone, Serialize, Deserialize)]
pub struct Slider {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub value: f64,
    pub min: f64,
    pub max: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub disabled: bool,
    #[serde(skip)]
    pub on_change: Option<Arc<dyn Fn(f64) + Send + Sync>>,
}

impl std::fmt::Debug for Slider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slider")
            .field("value", &self.value)
            .field("min", &self.min)
            .field("max", &self.max)
            .finish()
    }
}

impl Slider {
    pub fn new(value: f64) -> Self {
        Slider {
            id: None,
            value,
            min: 0.0,
            max: 100.0,
            step: None,
            label: None,
            disabled: false,
            on_change: None,
        }
    }

    pub fn min(mut self, min: f64) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: f64) -> Self {
        self.max = max;
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

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for Slider {
    fn widget_type(&self) -> &str {
        "slider"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "slider",
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

impl From<Slider> for Element {
    fn from(slider: Slider) -> Self {
        slider.into_element()
    }
}

/// A date picker. Values are ISO-8601 date strings (`YYYY-MM-DD`) rather than a
/// date type, so the framework stays free of a calendar dependency.
#[derive(Clone, Serialize, Deserialize)]
pub struct DateInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
    pub disabled: bool,
    #[serde(skip)]
    pub on_change: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl std::fmt::Debug for DateInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DateInput")
            .field("value", &self.value)
            .field("label", &self.label)
            .finish()
    }
}

impl DateInput {
    pub fn new() -> Self {
        DateInput {
            id: None,
            value: None,
            label: None,
            min: None,
            max: None,
            disabled: false,
            on_change: None,
        }
    }

    /// Set the selected date as an ISO-8601 `YYYY-MM-DD` string.
    pub fn value(mut self, value: &str) -> Self {
        self.value = Some(value.to_string());
        self
    }

    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn min(mut self, min: &str) -> Self {
        self.min = Some(min.to_string());
        self
    }

    pub fn max(mut self, max: &str) -> Self {
        self.max = Some(max.to_string());
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

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for DateInput {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetData for DateInput {
    fn widget_type(&self) -> &str {
        "date_input"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "date_input",
            "id": self.id,
            "value": self.value,
            "label": self.label,
            "min": self.min,
            "max": self.max,
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

impl From<DateInput> for Element {
    fn from(date_input: DateInput) -> Self {
        date_input.into_element()
    }
}

/// A colour picker. Values are CSS hex strings such as `#ff0000`.
#[derive(Clone, Serialize, Deserialize)]
pub struct ColorInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub disabled: bool,
    #[serde(skip)]
    pub on_change: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl std::fmt::Debug for ColorInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ColorInput")
            .field("value", &self.value)
            .field("label", &self.label)
            .finish()
    }
}

impl ColorInput {
    pub fn new() -> Self {
        ColorInput {
            id: None,
            value: None,
            label: None,
            disabled: false,
            on_change: None,
        }
    }

    /// Set the selected colour as a CSS hex string, e.g. `#3366ff`.
    pub fn value(mut self, value: &str) -> Self {
        self.value = Some(value.to_string());
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

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for ColorInput {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetData for ColorInput {
    fn widget_type(&self) -> &str {
        "color_input"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "color_input",
            "id": self.id,
            "value": self.value,
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
                    if let Some(value) = args.get("value").and_then(|v| v.as_str()) {
                        handler(value.to_string());
                    }
                }),
            );
        }
    }
}

impl From<ColorInput> for Element {
    fn from(color_input: ColorInput) -> Self {
        color_input.into_element()
    }
}

/// A set of mutually exclusive radio buttons, sharing [`SelectOption`] with
/// [`Select`].
#[derive(Clone, Serialize, Deserialize)]
pub struct RadioGroup {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub options: Vec<SelectOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub orientation: Orientation,
    pub disabled: bool,
    #[serde(skip)]
    pub on_change: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl std::fmt::Debug for RadioGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RadioGroup")
            .field("options", &self.options)
            .field("value", &self.value)
            .finish()
    }
}

impl RadioGroup {
    pub fn new(options: Vec<SelectOption>) -> Self {
        RadioGroup {
            id: None,
            options,
            value: None,
            label: None,
            orientation: Orientation::Vertical,
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

    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
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

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for RadioGroup {
    fn widget_type(&self) -> &str {
        "radio_group"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "radio_group",
            "id": self.id,
            "options": self.options,
            "value": self.value,
            "label": self.label,
            "orientation": self.orientation,
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

impl From<RadioGroup> for Element {
    fn from(radio_group: RadioGroup) -> Self {
        radio_group.into_element()
    }
}

/// A [`Select`] permitting several simultaneous selections.
#[derive(Clone, Serialize, Deserialize, Widget)]
pub struct MultiSelect {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    pub options: Vec<SelectOption>,
    #[prop]
    pub values: Vec<String>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[prop]
    pub disabled: bool,
    // The client sends every selected option as a JSON array in `value`,
    // matching the single-value shape of the other inputs. Deserializing into
    // `Vec<String>` rejects a scalar rather than coercing it to one element.
    #[event(arg = "value")]
    #[serde(skip)]
    pub on_change: Option<Arc<dyn Fn(Vec<String>) + Send + Sync>>,
}

impl std::fmt::Debug for MultiSelect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiSelect")
            .field("options", &self.options)
            .field("values", &self.values)
            .finish()
    }
}

impl MultiSelect {
    pub fn new(options: Vec<SelectOption>) -> Self {
        MultiSelect {
            id: None,
            options,
            values: Vec::new(),
            label: None,
            placeholder: None,
            disabled: false,
            on_change: None,
        }
    }

    pub fn values(mut self, values: Vec<String>) -> Self {
        self.values = values;
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

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(Vec<String>) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl From<MultiSelect> for Element {
    fn from(multi_select: MultiSelect) -> Self {
        multi_select.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use std::sync::Mutex;

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
        let mut element: Element = TextInput::new().into();
        element.assign_ids(&mut ctx);
        if let Element::Widget(ref w) = element {
            let json = w.to_json();
            assert_eq!(json["id"], "w-0");
            assert_eq!(json["type"], "text_input");
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_number_input_json_includes_id() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let mut element: Element = NumberInput::new().into();
        element.assign_ids(&mut ctx);
        if let Element::Widget(ref w) = element {
            let json = w.to_json();
            assert_eq!(json["id"], "w-0");
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_select_json_includes_id() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let mut element: Element = Select::new(vec![]).into();
        element.assign_ids(&mut ctx);
        if let Element::Widget(ref w) = element {
            let json = w.to_json();
            assert_eq!(json["id"], "w-0");
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_checkbox_json_includes_id() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let mut element: Element = Checkbox::new(false).into();
        element.assign_ids(&mut ctx);
        if let Element::Widget(ref w) = element {
            let json = w.to_json();
            assert_eq!(json["id"], "w-0");
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_widget_ids_are_sequential() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let mut el1: Element = TextInput::new().into();
        let mut el2: Element = TextInput::new().into();
        el1.assign_ids(&mut ctx);
        el2.assign_ids(&mut ctx);
        if let (Element::Widget(ref w1), Element::Widget(ref w2)) = (&el1, &el2) {
            assert_eq!(w1.get_id(), Some("w-0"));
            assert_eq!(w2.get_id(), Some("w-1"));
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_text_input_read_only() {
        assert!(!TextInput::new().read_only);
        let input = TextInput::new().read_only(true);
        assert!(input.read_only);
        assert_eq!(input.to_json()["readOnly"], true);
    }

    /// Register a widget's events and dispatch `event` with `args`, returning
    /// whether a handler ran. Every input below is a leaf, so it takes `w-0`.
    fn dispatch_on(
        widget: impl Into<Element>,
        event: &str,
        args: serde_json::Value,
    ) -> (bool, Element) {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let mut element: Element = widget.into();
        element.assign_ids(&mut ctx);
        let handled = ctx.take_event_registry().dispatch("w-0", event, args);
        (handled, element)
    }

    #[test]
    fn test_text_area_builder() {
        let area = TextArea::new()
            .value("Hello")
            .placeholder("Say something")
            .label("Message")
            .rows(6)
            .disabled(true);

        assert_eq!(area.value.as_deref(), Some("Hello"));
        assert_eq!(area.placeholder.as_deref(), Some("Say something"));
        assert_eq!(area.label.as_deref(), Some("Message"));
        assert_eq!(area.rows, Some(6));
        assert!(area.disabled);
    }

    #[test]
    fn test_text_area_json() {
        let json = TextArea::new()
            .value("Body")
            .rows(3)
            .on_change(|_| {})
            .to_json();
        assert_eq!(json["type"], "text_area");
        assert_eq!(json["value"], "Body");
        assert_eq!(json["rows"], 3);
        assert_eq!(json["hasOnChange"], true);
        assert_eq!(TextArea::new().to_json()["hasOnChange"], false);
    }

    #[test]
    fn test_text_area_change_dispatch() {
        let received = Arc::new(Mutex::new(None::<String>));
        let received_clone = received.clone();

        let (handled, _) = dispatch_on(
            TextArea::new().on_change(move |value| {
                *received_clone.lock().unwrap() = Some(value);
            }),
            "change",
            json!({"value": "typed"}),
        );

        assert!(handled);
        assert_eq!(received.lock().unwrap().as_deref(), Some("typed"));
    }

    #[test]
    fn test_slider_builder() {
        let slider = Slider::new(50.0)
            .min(10.0)
            .max(90.0)
            .step(5.0)
            .label("Volume");
        assert_eq!(slider.value, 50.0);
        assert_eq!(slider.min, 10.0);
        assert_eq!(slider.max, 90.0);
        assert_eq!(slider.step, Some(5.0));
        assert_eq!(slider.label.as_deref(), Some("Volume"));
    }

    #[test]
    fn test_slider_defaults_to_percent_range() {
        let slider = Slider::new(0.0);
        assert_eq!(slider.min, 0.0);
        assert_eq!(slider.max, 100.0);
        assert!(slider.step.is_none());
    }

    #[test]
    fn test_slider_json() {
        let json = Slider::new(25.0).max(50.0).on_change(|_| {}).to_json();
        assert_eq!(json["type"], "slider");
        assert_eq!(json["value"], 25.0);
        assert_eq!(json["max"], 50.0);
        assert_eq!(json["hasOnChange"], true);
    }

    #[test]
    fn test_slider_change_dispatch() {
        let received = Arc::new(Mutex::new(None::<f64>));
        let received_clone = received.clone();

        let (handled, _) = dispatch_on(
            Slider::new(0.0).on_change(move |value| {
                *received_clone.lock().unwrap() = Some(value);
            }),
            "change",
            json!({"value": 42.5}),
        );

        assert!(handled);
        assert_eq!(*received.lock().unwrap(), Some(42.5));
    }

    #[test]
    fn test_date_input_builder() {
        let date = DateInput::new()
            .value("2026-08-01")
            .min("2026-01-01")
            .max("2026-12-31")
            .label("Due");

        assert_eq!(date.value.as_deref(), Some("2026-08-01"));
        assert_eq!(date.min.as_deref(), Some("2026-01-01"));
        assert_eq!(date.max.as_deref(), Some("2026-12-31"));
        assert_eq!(date.label.as_deref(), Some("Due"));
    }

    #[test]
    fn test_date_input_json() {
        let json = DateInput::new()
            .value("2026-08-01")
            .on_change(|_| {})
            .to_json();
        assert_eq!(json["type"], "date_input");
        assert_eq!(json["value"], "2026-08-01");
        assert_eq!(json["hasOnChange"], true);
        assert!(DateInput::new().to_json()["value"].is_null());
    }

    #[test]
    fn test_date_input_change_dispatch() {
        let received = Arc::new(Mutex::new(None::<String>));
        let received_clone = received.clone();

        let (handled, _) = dispatch_on(
            DateInput::new().on_change(move |value| {
                *received_clone.lock().unwrap() = Some(value);
            }),
            "change",
            json!({"value": "2026-09-15"}),
        );

        assert!(handled);
        assert_eq!(received.lock().unwrap().as_deref(), Some("2026-09-15"));
    }

    #[test]
    fn test_color_input_builder() {
        let color = ColorInput::new().value("#3366ff").label("Accent");
        assert_eq!(color.value.as_deref(), Some("#3366ff"));
        assert_eq!(color.label.as_deref(), Some("Accent"));
        assert!(!color.disabled);
    }

    #[test]
    fn test_color_input_json() {
        let json = ColorInput::new()
            .value("#ff0000")
            .on_change(|_| {})
            .to_json();
        assert_eq!(json["type"], "color_input");
        assert_eq!(json["value"], "#ff0000");
        assert_eq!(json["hasOnChange"], true);
    }

    #[test]
    fn test_color_input_change_dispatch() {
        let received = Arc::new(Mutex::new(None::<String>));
        let received_clone = received.clone();

        let (handled, _) = dispatch_on(
            ColorInput::new().on_change(move |value| {
                *received_clone.lock().unwrap() = Some(value);
            }),
            "change",
            json!({"value": "#00ff00"}),
        );

        assert!(handled);
        assert_eq!(received.lock().unwrap().as_deref(), Some("#00ff00"));
    }

    fn radio_options() -> Vec<SelectOption> {
        vec![
            SelectOption {
                value: "s".into(),
                label: "Small".into(),
            },
            SelectOption {
                value: "l".into(),
                label: "Large".into(),
            },
        ]
    }

    #[test]
    fn test_radio_group_builder() {
        let group = RadioGroup::new(radio_options())
            .value("l")
            .label("Size")
            .orientation(Orientation::Horizontal);

        assert_eq!(group.options.len(), 2);
        assert_eq!(group.value.as_deref(), Some("l"));
        assert_eq!(group.orientation, Orientation::Horizontal);
    }

    #[test]
    fn test_radio_group_defaults_to_vertical() {
        assert_eq!(
            RadioGroup::new(radio_options()).orientation,
            Orientation::Vertical
        );
    }

    #[test]
    fn test_radio_group_json() {
        let json = RadioGroup::new(radio_options())
            .value("s")
            .on_change(|_| {})
            .to_json();

        assert_eq!(json["type"], "radio_group");
        assert_eq!(json["orientation"], "vertical");
        assert_eq!(json["options"][1]["label"], "Large");
        assert_eq!(json["value"], "s");
        assert_eq!(json["hasOnChange"], true);
    }

    #[test]
    fn test_radio_group_change_dispatch() {
        let received = Arc::new(Mutex::new(None::<String>));
        let received_clone = received.clone();

        let (handled, _) = dispatch_on(
            RadioGroup::new(radio_options()).on_change(move |value| {
                *received_clone.lock().unwrap() = Some(value);
            }),
            "change",
            json!({"value": "l"}),
        );

        assert!(handled);
        assert_eq!(received.lock().unwrap().as_deref(), Some("l"));
    }

    #[test]
    fn test_multi_select_builder() {
        let select = MultiSelect::new(radio_options())
            .values(vec!["s".into(), "l".into()])
            .placeholder("Pick sizes");

        assert_eq!(select.values, vec!["s".to_string(), "l".to_string()]);
        assert_eq!(select.placeholder.as_deref(), Some("Pick sizes"));
    }

    #[test]
    fn test_multi_select_json() {
        let json = MultiSelect::new(radio_options())
            .values(vec!["s".into()])
            .on_change(|_| {})
            .to_json();

        assert_eq!(json["type"], "multi_select");
        assert_eq!(json["values"][0], "s");
        assert_eq!(json["hasOnChange"], true);
        assert!(MultiSelect::new(vec![]).to_json()["values"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_multi_select_change_dispatch_decodes_array() {
        let received = Arc::new(Mutex::new(Vec::<String>::new()));
        let received_clone = received.clone();

        let (handled, _) = dispatch_on(
            MultiSelect::new(radio_options()).on_change(move |values| {
                *received_clone.lock().unwrap() = values;
            }),
            "change",
            json!({"value": ["s", "l"]}),
        );

        assert!(handled);
        assert_eq!(
            *received.lock().unwrap(),
            vec!["s".to_string(), "l".to_string()]
        );
    }

    #[test]
    fn test_multi_select_change_dispatch_ignores_scalar_value() {
        let hits = Arc::new(Mutex::new(0usize));
        let hits_clone = hits.clone();

        // The handler is registered, so dispatch reports a hit, but a non-array
        // payload must not be silently coerced into a one-element selection.
        let (handled, _) = dispatch_on(
            MultiSelect::new(radio_options()).on_change(move |_| {
                *hits_clone.lock().unwrap() += 1;
            }),
            "change",
            json!({"value": "s"}),
        );

        assert!(handled);
        assert_eq!(*hits.lock().unwrap(), 0);
    }

    #[test]
    fn test_new_inputs_accept_camel_case_event_names() {
        let received = Arc::new(Mutex::new(None::<String>));
        let received_clone = received.clone();

        let (handled, _) = dispatch_on(
            DateInput::new().on_change(move |value| {
                *received_clone.lock().unwrap() = Some(value);
            }),
            "onChange",
            json!({"value": "2026-08-01"}),
        );

        assert!(handled);
        assert_eq!(received.lock().unwrap().as_deref(), Some("2026-08-01"));
    }

    #[test]
    fn test_new_inputs_json_includes_assigned_ids() {
        let widgets: Vec<Element> = vec![
            TextArea::new().into(),
            Slider::new(1.0).into(),
            DateInput::new().into(),
            ColorInput::new().into(),
            RadioGroup::new(vec![]).into(),
            MultiSelect::new(vec![]).into(),
        ];

        for widget in widgets {
            let mut store = HookStore::default();
            let mut ctx = BuildContext::new(&mut store, None);
            let mut element = widget;
            element.assign_ids(&mut ctx);
            if let Element::Widget(ref w) = element {
                assert_eq!(w.to_json()["id"], "w-0", "widget {}", w.widget_type());
            } else {
                panic!("Expected Element::Widget");
            }
        }
    }
}
