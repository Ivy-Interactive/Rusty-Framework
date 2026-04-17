use crate::views::view::{BuildContext, Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// A modal dialog widget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dialog {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub open: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub children: Vec<Element>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<Vec<Element>>,
}

impl Dialog {
    pub fn new(open: bool) -> Self {
        Dialog {
            id: None,
            open,
            title: None,
            children: Vec::new(),
            footer: None,
        }
    }

    /// Assign a widget ID from the BuildContext.
    #[deprecated(note = "Widget IDs are now assigned automatically. Remove .build(ctx) calls.")]
    pub fn build(mut self, ctx: &mut BuildContext) -> Self {
        self.id = Some(ctx.next_widget_id());
        self
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn child(mut self, element: impl Into<Element>) -> Self {
        self.children.push(element.into());
        self
    }

    pub fn footer(mut self, elements: Vec<Element>) -> Self {
        self.footer = Some(elements);
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for Dialog {
    fn widget_type(&self) -> &str {
        "dialog"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "dialog",
            "id": self.id,
            "open": self.open,
            "title": self.title,
            "children": self.children.iter()
                .map(|c| serde_json::to_value(c).unwrap_or_default())
                .collect::<Vec<_>>(),
            "footer": self.footer.as_ref().map(|f| f.iter()
                .map(|c| serde_json::to_value(c).unwrap_or_default())
                .collect::<Vec<_>>()),
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

    fn children_mut(&mut self) -> Option<&mut Vec<Element>> {
        Some(&mut self.children)
    }

    fn footer_mut(&mut self) -> Option<&mut Vec<Element>> {
        self.footer.as_mut()
    }
}

impl From<Dialog> for Element {
    fn from(dialog: Dialog) -> Self {
        dialog.into_element()
    }
}
