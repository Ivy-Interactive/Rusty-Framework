use crate::shared::Size;
use crate::views::view::Element;
use rusty_macros::Widget;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// One panel of a horizontally-stacked drill-down, rendered by Ivy's
/// `BladeWidget`.
///
/// `index` is the blade's position in the stack, and `0` means the root: the
/// frontend hides the Close button when `index == 0`, so a root blade with an
/// `on_close` handler still cannot be closed by the user.
///
/// **Not covered here.** `BladeWidget` also renders a `BladeHeader` slot, which
/// Ivy fills with an `Ivy.Slot` child node -- Rust has no `Slot` widget, and
/// `processSlots` in `widgetRenderer.tsx` keys slots off `child.type ===
/// "Ivy.Slot"`, so a Rusty blade always renders `title` rather than a custom
/// header. Ivy's `UseBlades` controller is out of scope too: these widgets are
/// the render surface, and the push/pop stack belongs in the app's `use_state`.
///
/// **Inbound arg shape.** The Ivy frontend sends positional argument arrays
/// (`eventHandler("OnClose", id, [])`), while Rusty reads named keys out of an
/// args object. `on_close` and `on_refresh` take no argument, so they are
/// unaffected -- but see [`crate::widgets::Pagination`] for a handler where the
/// divergence is visible.
#[derive(Clone, Serialize, Deserialize, Widget)]
pub struct Blade {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    pub index: u32,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Size>,
    #[prop]
    #[children]
    pub children: Vec<Element>,
    #[event]
    #[serde(skip)]
    pub on_close: Option<Arc<dyn Fn() + Send + Sync>>,
    #[event]
    #[serde(skip)]
    pub on_refresh: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for Blade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Blade")
            .field("index", &self.index)
            .field("title", &self.title)
            .field("width", &self.width)
            .field("children", &self.children)
            .finish()
    }
}

impl Blade {
    /// A blade at `index` in the stack. `0` is the root blade.
    pub fn new(index: u32) -> Self {
        Blade {
            id: None,
            index,
            title: None,
            width: None,
            children: Vec::new(),
            on_close: None,
            on_refresh: None,
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    /// A fixed width. Without one the blade takes `flex-1` and shares the
    /// container's remaining space.
    pub fn width(mut self, width: Size) -> Self {
        self.width = Some(width);
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

    pub fn on_close(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_close = Some(Arc::new(handler));
        self
    }

    pub fn on_refresh(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_refresh = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl From<Blade> for Element {
    fn from(blade: Blade) -> Self {
        blade.into_element()
    }
}

/// The horizontal scroller that holds a stack of [`Blade`]s.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Widget)]
pub struct BladeContainer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    #[children]
    pub children: Vec<Element>,
}

impl BladeContainer {
    pub fn new() -> Self {
        BladeContainer::default()
    }

    /// Append a blade. The caller owns `index`, so a blade built in isolation
    /// and pushed later keeps its own number -- this never renumbers.
    pub fn blade(mut self, blade: Blade) -> Self {
        self.children.push(blade.into());
        self
    }

    /// A container accepts any element, not only [`Blade`]s.
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

impl From<BladeContainer> for Element {
    fn from(container: BladeContainer) -> Self {
        container.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::{BuildContext, WidgetData};
    use crate::widgets::TextBlock;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_blade_builder() {
        let blade = Blade::new(2)
            .title("Details")
            .width(Size::Px(320.0))
            .child(TextBlock::new("body"));

        assert_eq!(blade.index, 2);
        assert_eq!(blade.title.as_deref(), Some("Details"));
        assert_eq!(blade.width, Some(Size::Px(320.0)));
        assert_eq!(blade.children.len(), 1);
    }

    #[test]
    fn test_blade_defaults_have_no_handlers() {
        let blade = Blade::new(0);
        assert!(blade.title.is_none());
        assert!(blade.width.is_none());
        assert!(blade.on_close.is_none());
        assert!(blade.on_refresh.is_none());
    }

    #[test]
    fn test_blade_children_vec_builder() {
        let blade = Blade::new(0).children(vec![
            TextBlock::new("one").into(),
            TextBlock::new("two").into(),
        ]);
        assert_eq!(blade.children.len(), 2);
    }

    #[test]
    fn test_blade_json() {
        let json = Blade::new(1)
            .title("Details")
            .width(Size::Px(320.0))
            .child(TextBlock::new("body"))
            .on_close(|| {})
            .on_refresh(|| {})
            .to_json();

        assert_eq!(json["type"], "blade");
        assert_eq!(json["index"], 1);
        assert_eq!(json["title"], "Details");
        // Size serializes as a CSS length string, which is what getWidth() wants.
        assert_eq!(json["width"], "320px");
        assert_eq!(json["children"][0]["type"], "text_block");
        assert_eq!(json["hasOnClose"], true);
        assert_eq!(json["hasOnRefresh"], true);
    }

    #[test]
    fn test_blade_json_without_handlers() {
        let json = Blade::new(0).to_json();
        assert_eq!(json["hasOnClose"], false);
        assert_eq!(json["hasOnRefresh"], false);
        assert!(json["width"].is_null());
    }

    #[test]
    fn test_blade_container_json() {
        let json = BladeContainer::new()
            .blade(Blade::new(0).title("Root"))
            .blade(Blade::new(1).title("Child"))
            .to_json();

        assert_eq!(json["type"], "blade_container");
        assert_eq!(json["children"].as_array().unwrap().len(), 2);
        assert_eq!(json["children"][0]["index"], 0);
        assert_eq!(json["children"][1]["index"], 1);
    }

    #[test]
    fn test_blade_container_does_not_renumber() {
        // A blade built in isolation keeps the index its caller chose.
        let json = BladeContainer::new()
            .blade(Blade::new(7))
            .blade(Blade::new(3))
            .to_json();

        assert_eq!(json["children"][0]["index"], 7);
        assert_eq!(json["children"][1]["index"], 3);
    }

    #[test]
    fn test_blade_container_assign_ids_recurses() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = BladeContainer::new()
            .blade(Blade::new(0).child(TextBlock::new("a")))
            .blade(Blade::new(1).child(TextBlock::new("b")))
            .into();

        element.assign_ids(&mut ctx);

        let Element::Widget(ref widget) = element else {
            panic!("Expected Element::Widget");
        };
        let json = widget.to_json();
        assert_eq!(json["id"], "w-0");
        assert_eq!(json["children"][0]["id"], "w-1");
        assert_eq!(json["children"][0]["children"][0]["id"], "w-2");
        assert_eq!(json["children"][1]["id"], "w-3");
        assert_eq!(json["children"][1]["children"][0]["id"], "w-4");
    }

    #[test]
    fn test_blade_close_and_refresh_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let closes = Arc::new(AtomicUsize::new(0));
        let refreshes = Arc::new(AtomicUsize::new(0));
        let closes_clone = closes.clone();
        let refreshes_clone = refreshes.clone();

        let mut element: Element = Blade::new(1)
            .on_close(move || {
                closes_clone.fetch_add(1, Ordering::SeqCst);
            })
            .on_refresh(move || {
                refreshes_clone.fetch_add(1, Ordering::SeqCst);
            })
            .into();

        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        // Ivy sends "OnClose"; the e2e harness sends "close". Both resolve.
        assert!(registry.dispatch("w-0", "OnClose", serde_json::Value::Null));
        assert!(registry.dispatch("w-0", "close", serde_json::Value::Null));
        assert_eq!(closes.load(Ordering::SeqCst), 2);

        assert!(registry.dispatch("w-0", "OnRefresh", serde_json::Value::Null));
        assert!(registry.dispatch("w-0", "refresh", serde_json::Value::Null));
        assert_eq!(refreshes.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_blade_without_handler_has_no_registration() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = Blade::new(1).into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(!registry.dispatch("w-0", "close", serde_json::Value::Null));
        assert!(!registry.dispatch("w-0", "refresh", serde_json::Value::Null));
    }
}
