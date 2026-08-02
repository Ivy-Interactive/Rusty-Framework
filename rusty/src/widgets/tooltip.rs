use crate::views::view::{BuildContext, Element};
use rusty_macros::Widget;
use serde::{Deserialize, Serialize};

/// A hover tooltip widget that wraps a child element.
#[derive(Debug, Clone, Serialize, Deserialize, Widget)]
pub struct Tooltip {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    pub content: String,
    #[prop]
    #[child]
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

impl From<Tooltip> for Element {
    fn from(tooltip: Tooltip) -> Self {
        tooltip.into_element()
    }
}
