use crate::core::event_registry::EventRegistry;
use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// A collapsible section with a clickable header.
#[derive(Clone, Serialize, Deserialize)]
pub struct Expandable {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub title: String,
    pub expanded: bool,
    pub children: Vec<Element>,
    #[serde(skip)]
    pub on_toggle: Option<Arc<dyn Fn(bool) + Send + Sync>>,
}

impl std::fmt::Debug for Expandable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Expandable")
            .field("title", &self.title)
            .field("expanded", &self.expanded)
            .finish()
    }
}

impl Expandable {
    pub fn new(title: &str) -> Self {
        Expandable {
            id: None,
            title: title.to_string(),
            expanded: false,
            children: Vec::new(),
            on_toggle: None,
        }
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
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

    pub fn on_toggle(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
        self.on_toggle = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for Expandable {
    fn widget_type(&self) -> &str {
        "expandable"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "expandable",
            "id": self.id,
            "title": self.title,
            "expanded": self.expanded,
            "children": self.children.iter()
                .map(|c| serde_json::to_value(c).unwrap_or_default())
                .collect::<Vec<_>>(),
            "hasOnToggle": self.on_toggle.is_some(),
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
        if let Some(handler) = &self.on_toggle {
            let handler = handler.clone();
            registry.register(
                widget_id,
                "toggle",
                Arc::new(move |args| {
                    if let Some(value) = args.get("value").and_then(|v| v.as_bool()) {
                        handler(value);
                    }
                }),
            );
        }
    }

    fn children_mut(&mut self) -> Option<&mut Vec<Element>> {
        Some(&mut self.children)
    }
}

impl From<Expandable> for Element {
    fn from(expandable: Expandable) -> Self {
        expandable.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::BuildContext;
    use crate::widgets::text::TextBlock;
    use std::sync::Mutex;

    #[test]
    fn test_expandable_builder() {
        let expandable = Expandable::new("Details")
            .expanded(true)
            .child(TextBlock::new("Hidden content"));

        assert_eq!(expandable.title, "Details");
        assert!(expandable.expanded);
        assert_eq!(expandable.children.len(), 1);
    }

    #[test]
    fn test_expandable_json() {
        let json = Expandable::new("More")
            .child(TextBlock::new("Body"))
            .on_toggle(|_| {})
            .to_json();

        assert_eq!(json["type"], "expandable");
        assert_eq!(json["title"], "More");
        assert_eq!(json["expanded"], false);
        assert_eq!(json["hasOnToggle"], true);
        assert_eq!(json["children"][0]["content"], "Body");
    }

    #[test]
    fn test_expandable_json_without_handler() {
        assert_eq!(Expandable::new("X").to_json()["hasOnToggle"], false);
    }

    #[test]
    fn test_expandable_assign_ids_recurses_into_children() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = Expandable::new("Section")
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

    #[test]
    fn test_expandable_toggle_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received = Arc::new(Mutex::new(None::<bool>));
        let received_clone = received.clone();

        let mut element: Element = Expandable::new("Section")
            .on_toggle(move |value| {
                *received_clone.lock().unwrap() = Some(value);
            })
            .into();

        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "toggle", json!({"value": true})));
        assert_eq!(*received.lock().unwrap(), Some(true));
    }

    #[test]
    fn test_expandable_toggle_dispatch_accepts_camel_case() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received = Arc::new(Mutex::new(None::<bool>));
        let received_clone = received.clone();

        let mut element: Element = Expandable::new("Section")
            .expanded(true)
            .on_toggle(move |value| {
                *received_clone.lock().unwrap() = Some(value);
            })
            .into();

        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "onToggle", json!({"value": false})));
        assert_eq!(*received.lock().unwrap(), Some(false));
    }
}
