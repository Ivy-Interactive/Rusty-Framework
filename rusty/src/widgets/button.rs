use crate::core::event_registry::EventRegistry;
use crate::shared::{Color, Density, Icon};
use crate::views::view::{BuildContext, Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Outline,
    Ghost,
    Danger,
}

/// A clickable button widget.
#[derive(Clone, Serialize, Deserialize)]
pub struct Button {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<ButtonVariant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    pub disabled: bool,
    pub loading: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub density: Option<Density>,
    #[serde(skip)]
    pub on_click: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("title", &self.title)
            .field("variant", &self.variant)
            .field("disabled", &self.disabled)
            .finish()
    }
}

impl Button {
    pub fn new(title: &str) -> Self {
        Button {
            id: None,
            title: title.to_string(),
            variant: None,
            icon: None,
            disabled: false,
            loading: false,
            color: None,
            density: None,
            on_click: None,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn density(mut self, density: Density) -> Self {
        self.density = Some(density);
        self
    }

    pub fn on_click(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_click = Some(Arc::new(handler));
        self
    }

    /// Assign a widget ID from the BuildContext and register event handlers.
    #[deprecated(note = "Widget IDs are now assigned automatically. Remove .build(ctx) calls.")]
    pub fn build(mut self, ctx: &mut BuildContext) -> Self {
        let widget_id = ctx.next_widget_id();
        self.id = Some(widget_id.clone());
        if let Some(handler) = &self.on_click {
            let handler = handler.clone();
            ctx.register_event(&widget_id, "click", Arc::new(move |_args| handler()));
        }
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for Button {
    fn widget_type(&self) -> &str {
        "button"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "button",
            "id": self.id,
            "title": self.title,
            "variant": self.variant,
            "icon": self.icon,
            "disabled": self.disabled,
            "loading": self.loading,
            "color": self.color,
            "density": self.density,
            "hasOnClick": self.on_click.is_some(),
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
        if let Some(handler) = &self.on_click {
            let handler = handler.clone();
            registry.register(widget_id, "click", Arc::new(move |_args| handler()));
        }
    }
}

impl From<Button> for Element {
    fn from(button: Button) -> Self {
        button.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;

    #[test]
    fn test_button_builder() {
        let btn = Button::new("Click me")
            .variant(ButtonVariant::Primary)
            .disabled(false);

        assert_eq!(btn.title, "Click me");
        assert_eq!(btn.variant, Some(ButtonVariant::Primary));
        assert!(!btn.disabled);
    }

    #[test]
    fn test_button_serialization() {
        let btn = Button::new("Test");
        let json = btn.to_json();
        assert_eq!(json["type"], "button");
        assert_eq!(json["title"], "Test");
    }

    #[test]
    fn test_button_into_element() {
        let el: Element = Button::new("Click").into();
        assert!(matches!(el, Element::Widget(_)));
    }

    #[test]
    fn test_button_json_includes_id() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let btn = Button::new("Test").build(&mut ctx);
        let json = btn.to_json();
        assert_eq!(json["id"], "w-0");
        assert_eq!(json["type"], "button");
    }

    #[test]
    fn test_button_build_registers_handler() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let btn = Button::new("Click").on_click(|| {}).build(&mut ctx);
        assert_eq!(btn.id, Some("w-0".to_string()));
        assert!(json!({"hasOnClick": true})["hasOnClick"].as_bool().unwrap());
        let json = btn.to_json();
        assert_eq!(json["hasOnClick"], true);
    }
}
