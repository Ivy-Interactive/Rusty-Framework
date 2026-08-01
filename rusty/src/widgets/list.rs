use crate::core::event_registry::EventRegistry;
use crate::shared::Icon;
use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// A vertical list of [`ListItem`]s (or any other elements).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct List {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub items: Vec<Element>,
}

impl List {
    pub fn new() -> Self {
        List::default()
    }

    pub fn item(mut self, element: impl Into<Element>) -> Self {
        self.items.push(element.into());
        self
    }

    pub fn items(mut self, elements: Vec<Element>) -> Self {
        self.items.extend(elements);
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for List {
    fn widget_type(&self) -> &str {
        "list"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "list",
            "id": self.id,
            "items": self.items.iter()
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
        Some(&mut self.items)
    }
}

impl From<List> for Element {
    fn from(list: List) -> Self {
        list.into_element()
    }
}

/// A single row within a [`List`].
#[derive(Clone, Serialize, Deserialize)]
pub struct ListItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    #[serde(skip)]
    pub on_click: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for ListItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListItem")
            .field("title", &self.title)
            .field("subtitle", &self.subtitle)
            .finish()
    }
}

impl ListItem {
    pub fn new(title: &str) -> Self {
        ListItem {
            id: None,
            title: title.to_string(),
            subtitle: None,
            icon: None,
            on_click: None,
        }
    }

    pub fn subtitle(mut self, subtitle: &str) -> Self {
        self.subtitle = Some(subtitle.to_string());
        self
    }

    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn on_click(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_click = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for ListItem {
    fn widget_type(&self) -> &str {
        "list_item"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "list_item",
            "id": self.id,
            "title": self.title,
            "subtitle": self.subtitle,
            "icon": self.icon.as_ref().map(|i| i.0.clone()),
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
            registry.register(
                widget_id,
                "click",
                Arc::new(move |_args| {
                    handler();
                }),
            );
        }
    }
}

impl From<ListItem> for Element {
    fn from(item: ListItem) -> Self {
        item.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::BuildContext;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_list_builder() {
        let list = List::new()
            .item(ListItem::new("First"))
            .item(ListItem::new("Second"));
        assert_eq!(list.items.len(), 2);
    }

    #[test]
    fn test_list_items_vec_builder() {
        let list = List::new().items(vec![
            ListItem::new("One").into(),
            ListItem::new("Two").into(),
        ]);
        assert_eq!(list.items.len(), 2);
    }

    #[test]
    fn test_list_json() {
        let json = List::new().item(ListItem::new("Alpha")).to_json();
        assert_eq!(json["type"], "list");
        assert_eq!(json["items"][0]["type"], "list_item");
        assert_eq!(json["items"][0]["title"], "Alpha");
    }

    #[test]
    fn test_list_item_builder() {
        let item = ListItem::new("Inbox").subtitle("3 unread").icon("mail");
        assert_eq!(item.title, "Inbox");
        assert_eq!(item.subtitle.as_deref(), Some("3 unread"));
        assert_eq!(item.icon, Some(Icon::new("mail")));
    }

    #[test]
    fn test_list_item_json() {
        let json = ListItem::new("Inbox")
            .subtitle("3 unread")
            .icon("mail")
            .on_click(|| {})
            .to_json();

        assert_eq!(json["type"], "list_item");
        assert_eq!(json["title"], "Inbox");
        assert_eq!(json["subtitle"], "3 unread");
        assert_eq!(json["icon"], "mail");
        assert_eq!(json["hasOnClick"], true);
    }

    #[test]
    fn test_list_item_json_without_handler() {
        assert_eq!(ListItem::new("X").to_json()["hasOnClick"], false);
    }

    #[test]
    fn test_list_assign_ids_recurses_into_items() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = List::new()
            .item(ListItem::new("One"))
            .item(ListItem::new("Two"))
            .into();

        element.assign_ids(&mut ctx);

        if let Element::Widget(ref w) = element {
            let json = w.to_json();
            assert_eq!(json["id"], "w-0");
            assert_eq!(json["items"][0]["id"], "w-1");
            assert_eq!(json["items"][1]["id"], "w-2");
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_list_item_click_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let hits = Arc::new(AtomicUsize::new(0));
        let hits_clone = hits.clone();

        let mut element: Element = List::new()
            .item(ListItem::new("Clickable").on_click(move || {
                hits_clone.fetch_add(1, Ordering::SeqCst);
            }))
            .into();

        element.assign_ids(&mut ctx);

        // List is w-0, its single item w-1.
        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-1", "click", serde_json::Value::Null));
        assert!(registry.dispatch("w-1", "onClick", serde_json::Value::Null));
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }
}
