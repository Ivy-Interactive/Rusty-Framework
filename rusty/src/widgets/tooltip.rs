use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// A hover tooltip widget that wraps a child element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tooltip {
    pub content: String,
    pub child: Box<Element>,
}

impl Tooltip {
    pub fn new(content: &str, child: impl Into<Element>) -> Self {
        Tooltip {
            content: content.to_string(),
            child: Box::new(child.into()),
        }
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for Tooltip {
    fn widget_type(&self) -> &str {
        "tooltip"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "tooltip",
            "content": self.content,
            "child": serde_json::to_value(&*self.child).unwrap_or_default(),
        })
    }

    fn clone_box(&self) -> Box<dyn WidgetData> {
        Box::new(self.clone())
    }
}

impl From<Tooltip> for Element {
    fn from(tooltip: Tooltip) -> Self {
        tooltip.into_element()
    }
}
