use crate::shared::{Density, Icon};
use rusty_macros::Widget;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// One crumb of a [`Breadcrumbs`] trail.
///
/// A crumb is a *prop*, not a child widget, so it has no id of its own and
/// cannot carry a closure. The frontend fires a single `OnItemClick` on the
/// breadcrumbs widget with the crumb's index, and reads each crumb's
/// `hasOnClick` boolean to decide whether to render it as a button. Clickability
/// is therefore declared per item and handled once, on the widget.
///
/// The last crumb is never clickable: `BreadcrumbsWidget` treats it as the
/// current location regardless of `clickable`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreadcrumbItem {
    pub label: String,
    /// Whether the frontend renders this crumb as a clickable button. Named for
    /// the boolean the frontend reads, which follows Rusty's `has<Event>`
    /// convention even though the handler lives on the widget.
    #[serde(rename = "hasOnClick")]
    pub clickable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    pub disabled: bool,
}

impl BreadcrumbItem {
    /// A clickable crumb. `clickable` defaults to `true`, matching the common
    /// case of a navigable trail.
    pub fn new(label: &str) -> Self {
        BreadcrumbItem {
            label: label.to_string(),
            clickable: true,
            icon: None,
            tooltip: None,
            disabled: false,
        }
    }

    /// Render as plain text rather than a button, without disabling it.
    pub fn not_clickable(mut self) -> Self {
        self.clickable = false;
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

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A navigation trail of [`BreadcrumbItem`]s.
///
/// **Inbound arg shape.** `on_item_click` reads `args["index"]`, which is what
/// the e2e harness sends. Ivy's frontend sends a positional array
/// (`eventHandler("OnItemClick", id, [index])`) -- a pre-existing divergence
/// across every Rusty widget with a payload, documented in
/// [`crate::shared::ivy_node`], not something this widget resolves.
#[derive(Clone, Default, Serialize, Deserialize, Widget)]
pub struct Breadcrumbs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    pub items: Vec<BreadcrumbItem>,
    /// Rendered between crumbs. The frontend defaults to `/` when unset.
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
    #[prop]
    pub disabled: bool,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub density: Option<Density>,
    #[event(arg = "index")]
    #[serde(skip)]
    pub on_item_click: Option<Arc<dyn Fn(usize) + Send + Sync>>,
}

impl std::fmt::Debug for Breadcrumbs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Breadcrumbs")
            .field("items", &self.items)
            .field("separator", &self.separator)
            .field("disabled", &self.disabled)
            .field("density", &self.density)
            .finish()
    }
}

impl Breadcrumbs {
    pub fn new() -> Self {
        Breadcrumbs::default()
    }

    pub fn item(mut self, item: BreadcrumbItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: Vec<BreadcrumbItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn separator(mut self, separator: &str) -> Self {
        self.separator = Some(separator.to_string());
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

    /// Fired with the zero-based index of the clicked crumb.
    pub fn on_item_click(mut self, handler: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_item_click = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> crate::views::view::Element {
        crate::views::view::Element::Widget(Box::new(self))
    }
}

impl From<Breadcrumbs> for crate::views::view::Element {
    fn from(breadcrumbs: Breadcrumbs) -> Self {
        breadcrumbs.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::{BuildContext, Element, WidgetData};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    #[test]
    fn test_breadcrumb_item_defaults_to_clickable() {
        let item = BreadcrumbItem::new("Home");
        assert_eq!(item.label, "Home");
        assert!(item.clickable);
        assert!(!item.disabled);
        assert!(item.icon.is_none());
        assert!(item.tooltip.is_none());
    }

    #[test]
    fn test_breadcrumb_item_builder() {
        let item = BreadcrumbItem::new("Files")
            .icon("folder")
            .tooltip("Browse files")
            .disabled(true)
            .not_clickable();

        assert_eq!(item.icon, Some(Icon::new("folder")));
        assert_eq!(item.tooltip.as_deref(), Some("Browse files"));
        assert!(item.disabled);
        assert!(!item.clickable);
    }

    #[test]
    fn test_breadcrumbs_builder() {
        let crumbs = Breadcrumbs::new()
            .item(BreadcrumbItem::new("Home"))
            .items(vec![
                BreadcrumbItem::new("Projects"),
                BreadcrumbItem::new("Rusty"),
            ])
            .separator(">")
            .disabled(true)
            .density(Density::Compact);

        assert_eq!(crumbs.items.len(), 3);
        assert_eq!(crumbs.separator.as_deref(), Some(">"));
        assert!(crumbs.disabled);
        assert_eq!(crumbs.density, Some(Density::Compact));
    }

    #[test]
    fn test_breadcrumbs_json() {
        let json = Breadcrumbs::new()
            .item(BreadcrumbItem::new("Home").icon("home").tooltip("Start"))
            .item(BreadcrumbItem::new("Current").not_clickable())
            .separator(">")
            .density(Density::Comfortable)
            .on_item_click(|_| {})
            .to_json();

        assert_eq!(json["type"], "breadcrumbs");
        assert_eq!(json["separator"], ">");
        assert_eq!(json["disabled"], false);
        assert_eq!(json["density"], "comfortable");
        assert_eq!(json["hasOnItemClick"], true);

        // The frontend reads a per-item `hasOnClick`, not a per-item handler.
        assert_eq!(json["items"][0]["label"], "Home");
        assert_eq!(json["items"][0]["hasOnClick"], true);
        assert_eq!(json["items"][0]["icon"], "home");
        assert_eq!(json["items"][0]["tooltip"], "Start");
        assert_eq!(json["items"][1]["hasOnClick"], false);
        assert_eq!(json["items"][1]["disabled"], false);
    }

    #[test]
    fn test_breadcrumbs_json_without_handler() {
        let json = Breadcrumbs::new().to_json();
        assert_eq!(json["hasOnItemClick"], false);
        assert_eq!(json["items"].as_array().unwrap().len(), 0);
        // Unset separator stays null so the frontend applies its own "/" default.
        assert!(json["separator"].is_null());
        assert!(json["density"].is_null());
    }

    #[test]
    fn test_breadcrumbs_item_click_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let clicks: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
        let clicks_clone = clicks.clone();

        let mut element: Element = Breadcrumbs::new()
            .item(BreadcrumbItem::new("Home"))
            .item(BreadcrumbItem::new("Projects"))
            .item(BreadcrumbItem::new("Rusty"))
            .on_item_click(move |index| {
                clicks_clone.lock().unwrap().push(index);
            })
            .into();

        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "onItemClick", serde_json::json!({"index": 2})));
        assert!(registry.dispatch("w-0", "OnItemClick", serde_json::json!({"index": 0})));
        assert_eq!(*clicks.lock().unwrap(), vec![2, 0]);
    }

    #[test]
    fn test_breadcrumbs_malformed_payload_is_dropped() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let hits = Arc::new(AtomicUsize::new(0));
        let hits_clone = hits.clone();

        let mut element: Element = Breadcrumbs::new()
            .item(BreadcrumbItem::new("Home"))
            .on_item_click(move |_| {
                hits_clone.fetch_add(1, Ordering::SeqCst);
            })
            .into();

        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        // The registration exists, so dispatch reports true -- but a payload that
        // does not deserialize into usize must not reach the handler.
        assert!(registry.dispatch("w-0", "itemclick", serde_json::json!({"index": "two"})));
        assert!(registry.dispatch("w-0", "itemclick", serde_json::json!({})));
        assert_eq!(hits.load(Ordering::SeqCst), 0);
    }
}
