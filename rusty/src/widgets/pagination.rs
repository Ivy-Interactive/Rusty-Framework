use crate::shared::Density;
use crate::views::view::Element;
use rusty_macros::Widget;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A page selector with prev/next arrows, boundary pages and ellipsis gaps.
///
/// `page` is **1-based**, as `PaginationWidget` treats it: page `1` disables the
/// Previous arrow and page `num_pages` disables Next. `0` means "no page
/// selected" -- the frontend tests `!page` and disables both arrows and every
/// sibling, leaving only the boundary pages visible.
///
/// **Inbound arg shape.** `on_change` reads `args["value"]`, which is what the
/// e2e harness sends. Ivy's frontend sends a positional array
/// (`eventHandler("OnChange", id, [page])`) -- a pre-existing divergence across
/// every Rusty widget with a payload, documented in
/// [`crate::shared::ivy_node`], not something this widget resolves.
#[derive(Clone, Serialize, Deserialize, Widget)]
pub struct Pagination {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The selected page, 1-based. `0` selects nothing.
    #[prop]
    pub page: u32,
    /// Total page count. Serializes as `numPages`.
    #[prop]
    pub num_pages: u32,
    /// How many pages to show either side of the current one.
    #[prop]
    pub siblings: u32,
    /// How many pages to always show at each end.
    #[prop]
    pub boundaries: u32,
    #[prop]
    pub disabled: bool,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub density: Option<Density>,
    #[event(arg = "value")]
    #[serde(skip)]
    pub on_change: Option<Arc<dyn Fn(u32) + Send + Sync>>,
}

impl std::fmt::Debug for Pagination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pagination")
            .field("page", &self.page)
            .field("num_pages", &self.num_pages)
            .field("siblings", &self.siblings)
            .field("boundaries", &self.boundaries)
            .field("disabled", &self.disabled)
            .field("density", &self.density)
            .finish()
    }
}

impl Pagination {
    /// `page` is 1-based. `siblings` and `boundaries` default to `1`, matching
    /// the frontend's own defaults.
    pub fn new(page: u32, num_pages: u32) -> Self {
        Pagination {
            id: None,
            page,
            num_pages,
            siblings: 1,
            boundaries: 1,
            disabled: false,
            density: None,
            on_change: None,
        }
    }

    pub fn siblings(mut self, siblings: u32) -> Self {
        self.siblings = siblings;
        self
    }

    pub fn boundaries(mut self, boundaries: u32) -> Self {
        self.boundaries = boundaries;
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

    /// Fired with the newly selected 1-based page.
    pub fn on_change(mut self, handler: impl Fn(u32) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl From<Pagination> for Element {
    fn from(pagination: Pagination) -> Self {
        pagination.into_element()
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
    fn test_pagination_defaults_match_the_frontend() {
        let pagination = Pagination::new(1, 10);
        assert_eq!(pagination.page, 1);
        assert_eq!(pagination.num_pages, 10);
        assert_eq!(pagination.siblings, 1);
        assert_eq!(pagination.boundaries, 1);
        assert!(!pagination.disabled);
        assert!(pagination.density.is_none());
    }

    #[test]
    fn test_pagination_builder() {
        let pagination = Pagination::new(3, 20)
            .siblings(2)
            .boundaries(3)
            .disabled(true)
            .density(Density::Compact);

        assert_eq!(pagination.siblings, 2);
        assert_eq!(pagination.boundaries, 3);
        assert!(pagination.disabled);
        assert_eq!(pagination.density, Some(Density::Compact));
    }

    #[test]
    fn test_pagination_json() {
        let json = Pagination::new(3, 20)
            .siblings(2)
            .density(Density::Normal)
            .on_change(|_| {})
            .to_json();

        assert_eq!(json["type"], "pagination");
        assert_eq!(json["page"], 3);
        // The frontend prop is `numPages`; the derive camelCases the field name.
        assert_eq!(json["numPages"], 20);
        assert_eq!(json["siblings"], 2);
        assert_eq!(json["boundaries"], 1);
        assert_eq!(json["disabled"], false);
        assert_eq!(json["density"], "normal");
        assert_eq!(json["hasOnChange"], true);
    }

    #[test]
    fn test_pagination_json_without_handler() {
        let json = Pagination::new(0, 5).to_json();
        assert_eq!(json["hasOnChange"], false);
        // 0 means "no page selected" to the frontend, and travels as 0 not null.
        assert_eq!(json["page"], 0);
        assert!(json["density"].is_null());
    }

    #[test]
    fn test_pagination_change_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let pages: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
        let pages_clone = pages.clone();

        let mut element: Element = Pagination::new(1, 10)
            .on_change(move |page| {
                pages_clone.lock().unwrap().push(page);
            })
            .into();

        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "change", serde_json::json!({"value": 3})));
        assert!(registry.dispatch("w-0", "OnChange", serde_json::json!({"value": 7})));
        assert_eq!(*pages.lock().unwrap(), vec![3, 7]);
    }

    #[test]
    fn test_pagination_malformed_payload_is_dropped() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let hits = Arc::new(AtomicUsize::new(0));
        let hits_clone = hits.clone();

        let mut element: Element = Pagination::new(1, 10)
            .on_change(move |_| {
                hits_clone.fetch_add(1, Ordering::SeqCst);
            })
            .into();

        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        // A negative page cannot deserialize into u32, so the handler is skipped.
        assert!(registry.dispatch("w-0", "change", serde_json::json!({"value": -1})));
        assert!(registry.dispatch("w-0", "change", serde_json::json!({"page": 3})));
        assert_eq!(hits.load(Ordering::SeqCst), 0);
    }
}
