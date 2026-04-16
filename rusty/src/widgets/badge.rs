use crate::shared::Color;
use crate::views::view::{Element, WidgetData};
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
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<BadgeVariant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}

impl Badge {
    pub fn new(label: &str) -> Self {
        Badge {
            label: label.to_string(),
            variant: None,
            color: None,
        }
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
            "label": self.label,
            "variant": self.variant,
            "color": self.color,
        })
    }

    fn clone_box(&self) -> Box<dyn WidgetData> {
        Box::new(self.clone())
    }
}

impl From<Badge> for Element {
    fn from(badge: Badge) -> Self {
        badge.into_element()
    }
}
