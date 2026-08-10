use crate::shared::{Density, Icon};
use crate::views::view::Element;
use rusty_macros::Widget;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// What a [`ToolbarItem`] renders as.
///
/// Serializes **PascalCase** -- deliberately no `rename_all` -- because
/// `ToolbarWidget` compares `item.variant === "Group"` / `"Separator"` on a
/// *nested* value, and `ivy_node`'s `ENUM_PROPS` recasing only reaches top-level
/// props.
///
/// Ivy's `MenuItem` type also admits `Checkbox` and `Radio` variants, but
/// `ToolbarWidget` branches on neither -- both fall through to the default
/// button -- so they are omitted rather than modelled as dead weight.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolbarItemVariant {
    #[default]
    Default,
    Separator,
    Group,
}

/// One entry of a [`Toolbar`].
///
/// Items are props, not child widgets: they carry no id, and `children` is
/// `Vec<ToolbarItem>` rather than `Vec<Element>`, so nesting a group does not
/// need the derive's `children_mut` machinery.
///
/// `tag` is what identifies the item to the `on_select` handler. An item without
/// one is inert -- `ToolbarWidget` returns early from its click handler when
/// `item.tag` is unset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolbarItem {
    pub variant: ToolbarItemVariant,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    pub checked: bool,
    pub disabled: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<ToolbarItem>,
}

impl ToolbarItem {
    /// A clickable button identified by `tag`, which is the value `on_select`
    /// receives.
    pub fn button(tag: &str) -> Self {
        ToolbarItem {
            variant: ToolbarItemVariant::Default,
            label: None,
            icon: None,
            tag: Some(tag.to_string()),
            tooltip: None,
            checked: false,
            disabled: false,
            children: Vec::new(),
        }
    }

    /// A vertical rule between items. Carries no tag and fires nothing.
    pub fn separator() -> Self {
        ToolbarItem {
            variant: ToolbarItemVariant::Separator,
            label: None,
            icon: None,
            tag: None,
            tooltip: None,
            checked: false,
            disabled: false,
            children: Vec::new(),
        }
    }

    /// A labelled cluster of items. Add members with [`ToolbarItem::item`].
    pub fn group(label: &str) -> Self {
        ToolbarItem {
            variant: ToolbarItemVariant::Group,
            label: Some(label.to_string()),
            icon: None,
            tag: None,
            tooltip: None,
            checked: false,
            disabled: false,
            children: Vec::new(),
        }
    }

    /// Append a nested item. Only a `Group` renders its children.
    pub fn item(mut self, item: ToolbarItem) -> Self {
        self.children.push(item);
        self
    }

    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn tooltip(mut self, tooltip: &str) -> Self {
        self.tooltip = Some(tooltip.to_string());
        self
    }

    /// Renders the button with an accent background, for a toggled tool.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A horizontal bar of buttons, separators and groups.
///
/// **Inbound arg shape.** `on_select` reads `args["tag"]`, which is what the
/// e2e harness sends. Ivy's frontend sends a positional array
/// (`eventHandler("OnSelect", id, [item.tag])`) -- a pre-existing divergence
/// across every Rusty widget with a payload, documented in
/// [`crate::shared::ivy_node`], not something this widget resolves.
#[derive(Clone, Default, Serialize, Deserialize, Widget)]
pub struct Toolbar {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    pub items: Vec<ToolbarItem>,
    #[prop]
    pub disabled: bool,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub density: Option<Density>,
    #[event(arg = "tag")]
    #[serde(skip)]
    pub on_select: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl std::fmt::Debug for Toolbar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Toolbar")
            .field("items", &self.items)
            .field("disabled", &self.disabled)
            .field("density", &self.density)
            .finish()
    }
}

impl Toolbar {
    pub fn new() -> Self {
        Toolbar::default()
    }

    pub fn item(mut self, item: ToolbarItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: Vec<ToolbarItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn density(mut self, density: Density) -> Self {
        self.density = Some(density);
        self
    }

    /// Fired with the `tag` of the selected item, at any nesting depth.
    pub fn on_select(mut self, handler: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_select = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl From<Toolbar> for Element {
    fn from(toolbar: Toolbar) -> Self {
        toolbar.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::{BuildContext, WidgetData};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    #[test]
    fn test_toolbar_item_constructors() {
        let button = ToolbarItem::button("save");
        assert_eq!(button.variant, ToolbarItemVariant::Default);
        assert_eq!(button.tag.as_deref(), Some("save"));

        let separator = ToolbarItem::separator();
        assert_eq!(separator.variant, ToolbarItemVariant::Separator);
        assert!(separator.tag.is_none());

        let group = ToolbarItem::group("Edit");
        assert_eq!(group.variant, ToolbarItemVariant::Group);
        assert_eq!(group.label.as_deref(), Some("Edit"));
        assert!(group.children.is_empty());
    }

    #[test]
    fn test_toolbar_item_builder() {
        let item = ToolbarItem::button("bold")
            .label("Bold")
            .icon("bold")
            .tooltip("Bold (Ctrl+B)")
            .checked(true)
            .disabled(true);

        assert_eq!(item.label.as_deref(), Some("Bold"));
        assert_eq!(item.icon, Some(Icon::new("bold")));
        assert_eq!(item.tooltip.as_deref(), Some("Bold (Ctrl+B)"));
        assert!(item.checked);
        assert!(item.disabled);
    }

    #[test]
    fn test_toolbar_builder() {
        let toolbar = Toolbar::new()
            .item(ToolbarItem::button("save"))
            .items(vec![
                ToolbarItem::separator(),
                ToolbarItem::group("Edit").item(ToolbarItem::button("cut")),
            ])
            .disabled(true)
            .density(Density::Comfortable);

        assert_eq!(toolbar.items.len(), 3);
        assert_eq!(toolbar.items[2].children.len(), 1);
        assert!(toolbar.disabled);
        assert_eq!(toolbar.density, Some(Density::Comfortable));
    }

    #[test]
    fn test_toolbar_json() {
        let json = Toolbar::new()
            .item(ToolbarItem::button("save").label("Save").icon("save"))
            .item(ToolbarItem::separator())
            .item(
                ToolbarItem::group("Edit")
                    .item(ToolbarItem::button("cut").label("Cut"))
                    .item(ToolbarItem::button("copy").label("Copy").checked(true)),
            )
            .density(Density::Compact)
            .on_select(|_| {})
            .to_json();

        assert_eq!(json["type"], "toolbar");
        assert_eq!(json["disabled"], false);
        assert_eq!(json["density"], "compact");
        assert_eq!(json["hasOnSelect"], true);

        // The variant is PascalCase because ToolbarWidget compares it verbatim on
        // a nested value, where ivy_node's recasing never reaches.
        assert_eq!(json["items"][0]["variant"], "Default");
        assert_eq!(json["items"][0]["tag"], "save");
        assert_eq!(json["items"][0]["icon"], "save");
        assert_eq!(json["items"][1]["variant"], "Separator");
        assert_eq!(json["items"][2]["variant"], "Group");
        assert_eq!(json["items"][2]["label"], "Edit");
        assert_eq!(json["items"][2]["children"][1]["label"], "Copy");
        assert_eq!(json["items"][2]["children"][1]["checked"], true);
    }

    #[test]
    fn test_toolbar_json_omits_empty_children_and_unset_fields() {
        let json = Toolbar::new().item(ToolbarItem::separator()).to_json();

        assert_eq!(json["hasOnSelect"], false);
        let item = &json["items"][0];
        assert!(item.get("children").is_none(), "{:?}", item);
        assert!(item.get("tag").is_none());
        assert!(item.get("label").is_none());
        assert!(item.get("icon").is_none());
    }

    #[test]
    fn test_toolbar_select_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let tags: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let tags_clone = tags.clone();

        let mut element: Element = Toolbar::new()
            .item(ToolbarItem::button("save"))
            .on_select(move |tag| {
                tags_clone.lock().unwrap().push(tag);
            })
            .into();

        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "OnSelect", serde_json::json!({"tag": "save"})));
        assert!(registry.dispatch("w-0", "select", serde_json::json!({"tag": "cut"})));
        assert_eq!(*tags.lock().unwrap(), vec!["save", "cut"]);
    }

    #[test]
    fn test_toolbar_malformed_payload_is_dropped() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let hits = Arc::new(AtomicUsize::new(0));
        let hits_clone = hits.clone();

        let mut element: Element = Toolbar::new()
            .item(ToolbarItem::button("save"))
            .on_select(move |_| {
                hits_clone.fetch_add(1, Ordering::SeqCst);
            })
            .into();

        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "select", serde_json::json!({"tag": 7})));
        assert!(registry.dispatch("w-0", "select", serde_json::json!({})));
        assert_eq!(hits.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_toolbar_items_are_props_not_children() {
        // Items carry no ids of their own: assign_ids stops at the toolbar.
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = Toolbar::new()
            .item(ToolbarItem::button("save"))
            .item(ToolbarItem::group("Edit").item(ToolbarItem::button("cut")))
            .into();

        element.assign_ids(&mut ctx);

        let Element::Widget(ref widget) = element else {
            panic!("Expected Element::Widget");
        };
        let json = widget.to_json();
        assert_eq!(json["id"], "w-0");
        assert!(json["items"][0].get("id").is_none());
        assert!(json["items"][1]["children"][0].get("id").is_none());
    }
}
