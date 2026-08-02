use crate::shared::Icon;
use crate::views::view::Element;
use rusty_macros::Widget;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A vertical list of [`ListItem`]s (or any other elements).
#[derive(Debug, Clone, Default, Serialize, Deserialize, Widget)]
pub struct List {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    #[children]
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

impl From<List> for Element {
    fn from(list: List) -> Self {
        list.into_element()
    }
}

/// A single row within a [`List`].
#[derive(Clone, Serialize, Deserialize, Widget)]
pub struct ListItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    pub title: String,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    #[event]
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

impl From<ListItem> for Element {
    fn from(item: ListItem) -> Self {
        item.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::{BuildContext, WidgetData};
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
