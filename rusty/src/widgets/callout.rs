use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Severity styling for a [`Callout`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CalloutVariant {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

/// A highlighted message block with an optional title.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Callout {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub variant: CalloutVariant,
    pub children: Vec<Element>,
}

impl Callout {
    pub fn new() -> Self {
        Callout::default()
    }

    pub fn info() -> Self {
        Callout::with_variant(CalloutVariant::Info)
    }

    pub fn success() -> Self {
        Callout::with_variant(CalloutVariant::Success)
    }

    pub fn warning() -> Self {
        Callout::with_variant(CalloutVariant::Warning)
    }

    pub fn error() -> Self {
        Callout::with_variant(CalloutVariant::Error)
    }

    fn with_variant(variant: CalloutVariant) -> Self {
        Callout {
            variant,
            ..Callout::default()
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn variant(mut self, variant: CalloutVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn child(mut self, element: impl Into<Element>) -> Self {
        self.children.push(element.into());
        self
    }

    pub fn children(mut self, elements: Vec<Element>) -> Self {
        self.children.extend(elements);
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for Callout {
    fn widget_type(&self) -> &str {
        "callout"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "callout",
            "id": self.id,
            "title": self.title,
            "variant": self.variant,
            "children": self.children.iter()
                .map(|c| serde_json::to_value(c).unwrap_or_default())
                .collect::<Vec<_>>(),
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
}

impl From<Callout> for Element {
    fn from(callout: Callout) -> Self {
        callout.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::BuildContext;
    use crate::widgets::text::TextBlock;

    #[test]
    fn test_callout_builder() {
        let callout = Callout::warning()
            .title("Heads up")
            .child(TextBlock::new("Disk is nearly full"));

        assert_eq!(callout.variant, CalloutVariant::Warning);
        assert_eq!(callout.title.as_deref(), Some("Heads up"));
        assert_eq!(callout.children.len(), 1);
    }

    #[test]
    fn test_callout_variant_constructors() {
        assert_eq!(Callout::info().variant, CalloutVariant::Info);
        assert_eq!(Callout::success().variant, CalloutVariant::Success);
        assert_eq!(Callout::warning().variant, CalloutVariant::Warning);
        assert_eq!(Callout::error().variant, CalloutVariant::Error);
        // `new()` defaults to Info.
        assert_eq!(Callout::new().variant, CalloutVariant::Info);
    }

    #[test]
    fn test_callout_json() {
        let json = Callout::error()
            .title("Failed")
            .child(TextBlock::new("Try again"))
            .to_json();

        assert_eq!(json["type"], "callout");
        assert_eq!(json["variant"], "error");
        assert_eq!(json["title"], "Failed");
        assert_eq!(json["children"][0]["content"], "Try again");
    }

    #[test]
    fn test_callout_assign_ids_recurses_into_children() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = Callout::info()
            .child(TextBlock::new("One"))
            .child(TextBlock::new("Two"))
            .into();

        element.assign_ids(&mut ctx);

        if let Element::Widget(ref w) = element {
            let json = w.to_json();
            assert_eq!(json["id"], "w-0");
            assert_eq!(json["children"][0]["id"], "w-1");
            assert_eq!(json["children"][1]["id"], "w-2");
        } else {
            panic!("Expected Element::Widget");
        }
    }
}
