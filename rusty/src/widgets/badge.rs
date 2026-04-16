use crate::shared::Color;
use crate::views::view::{BuildContext, Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BadgeVariant {
    Default,
    Outline,
    Dot,
}

/// A status indicator badge widget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Badge {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<BadgeVariant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}

impl Badge {
    pub fn new(label: &str) -> Self {
        Badge {
            id: None,
            label: label.to_string(),
            variant: None,
            color: None,
        }
    }

    /// Assign a widget ID from the BuildContext.
    #[deprecated(note = "Widget IDs are now assigned automatically. Remove .build(ctx) calls.")]
    pub fn build(mut self, ctx: &mut BuildContext) -> Self {
        self.id = Some(ctx.next_widget_id());
        self
    }

    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for Badge {
    fn widget_type(&self) -> &str {
        "badge"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "badge",
            "id": self.id,
            "label": self.label,
            "variant": self.variant,
            "color": self.color,
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
}

impl From<Badge> for Element {
    fn from(badge: Badge) -> Self {
        badge.into_element()
    }
}
