use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// A flex-grow filler that pushes surrounding siblings apart.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Spacer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl Spacer {
    pub fn new() -> Self {
        Spacer { id: None }
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for Spacer {
    fn widget_type(&self) -> &str {
        "spacer"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "spacer",
            "id": self.id,
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

impl From<Spacer> for Element {
    fn from(spacer: Spacer) -> Self {
        spacer.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::BuildContext;

    #[test]
    fn test_spacer_builder() {
        let spacer = Spacer::new();
        assert!(spacer.id.is_none());
    }

    #[test]
    fn test_spacer_json() {
        let json = Spacer::new().to_json();
        assert_eq!(json["type"], "spacer");
        assert!(json["id"].is_null());
    }

    #[test]
    fn test_spacer_assign_ids() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let mut element: Element = Spacer::new().into();
        element.assign_ids(&mut ctx);
        if let Element::Widget(ref w) = element {
            assert_eq!(w.get_id(), Some("w-0"));
            assert_eq!(w.to_json()["id"], "w-0");
        } else {
            panic!("Expected Element::Widget");
        }
    }
}
