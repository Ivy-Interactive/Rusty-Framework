use crate::views::view::{BuildContext, Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// A hover tooltip widget that wraps a child element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tooltip {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub content: String,
    pub child: Box<Element>,
}

impl Tooltip {
    pub fn new(content: &str, child: impl Into<Element>) -> Self {
        Tooltip {
            id: None,
            content: content.to_string(),
            child: Box::new(child.into()),
        }
    }

    /// Assign a widget ID from the BuildContext.
    #[deprecated(note = "Widget IDs are now assigned automatically. Remove .build(ctx) calls.")]
    pub fn build(mut self, ctx: &mut BuildContext) -> Self {
        self.id = Some(ctx.next_widget_id());
        self
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
            "id": self.id,
            "content": self.content,
            "child": serde_json::to_value(&*self.child).unwrap_or_default(),
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

    fn single_child_mut(&mut self) -> Option<&mut Element> {
        Some(&mut self.child)
    }
}

impl From<Tooltip> for Element {
    fn from(tooltip: Tooltip) -> Self {
        tooltip.into_element()
    }
}
