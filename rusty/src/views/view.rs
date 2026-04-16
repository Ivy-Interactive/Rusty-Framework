use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::core::event_registry::{EventCallback, EventRegistry};
use crate::hooks::hook_store::HookStore;
use crate::shared::ViewId;

/// Trait for serializable UI widgets sent to the client.
pub trait Widget: Send + Sync + Debug + 'static {
    /// The widget type name (e.g., "button", "text_block").
    fn widget_type(&self) -> &str;

    /// Serialize to a JSON value for the frontend.
    fn to_json(&self) -> serde_json::Value;

    fn as_any(&self) -> &dyn Any;
}

/// The element tree produced by View::build.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Element {
    #[serde(rename = "widget")]
    Widget(Box<dyn WidgetData>),
    #[serde(rename = "fragment")]
    Fragment(Vec<Element>),
    #[serde(rename = "empty")]
    Empty,
}

/// Type-erased widget data for serialization.
pub trait WidgetData: Send + Sync + Debug + 'static {
    fn widget_type(&self) -> &str;
    fn to_json(&self) -> serde_json::Value;
    fn clone_box(&self) -> Box<dyn WidgetData>;

    /// Set the widget's ID. Used by the automatic ID assignment tree walk.
    fn assign_id(&mut self, id: String);

    /// Get the widget's current ID, if assigned.
    fn get_id(&self) -> Option<&str>;

    /// Register this widget's event handlers into the registry.
    /// Called automatically during the post-build tree walk.
    fn register_events(&self, _widget_id: &str, _registry: &mut EventRegistry) {}

    /// Return mutable references to child elements for recursive tree walking.
    /// Container widgets (Layout, Card, Dialog) override this.
    fn children_mut(&mut self) -> Option<&mut Vec<Element>> {
        None
    }

    /// Return a mutable reference to a single child element.
    /// Tooltip overrides this for its single wrapped child.
    fn single_child_mut(&mut self) -> Option<&mut Element> {
        None
    }
}

impl Clone for Box<dyn WidgetData> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl Serialize for Box<dyn WidgetData> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_json().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Box<dyn WidgetData> {
    fn deserialize<D: serde::Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        // Widget deserialization requires a registry; placeholder for now
        Err(serde::de::Error::custom(
            "Widget deserialization not yet supported",
        ))
    }
}

impl Element {
    /// Recursively assign widget IDs and register event handlers for all widgets
    /// in the element tree that don't already have an ID.
    /// This mirrors Ivy Framework's automatic ID assignment in `WidgetTree.BuildWidget()`.
    pub fn assign_ids(&mut self, ctx: &mut BuildContext) {
        match self {
            Element::Widget(widget) => {
                if widget.get_id().is_none() {
                    let id = ctx.next_widget_id();
                    widget.assign_id(id.clone());
                    widget.register_events(&id, &mut ctx.event_registry);
                }
                if let Some(children) = widget.children_mut() {
                    for child in children {
                        child.assign_ids(ctx);
                    }
                }
                if let Some(child) = widget.single_child_mut() {
                    child.assign_ids(ctx);
                }
            }
            Element::Fragment(children) => {
                for child in children {
                    child.assign_ids(ctx);
                }
            }
            Element::Empty => {}
        }
    }
}

/// A snapshot of a view's context map, cheaply clonable via `Arc`.
pub type ContextSnapshot = HashMap<TypeId, Arc<dyn Any + Send + Sync>>;

/// A stateful component that produces an element tree.
pub trait View: Send + Sync + 'static {
    /// Build the element tree for this view.
    fn build(&self, ctx: &mut BuildContext) -> Element;
}

/// Context passed to View::build providing access to hooks, state, and event registration.
///
/// Holds a mutable reference to a `HookStore` that persists across re-renders,
/// analogous to Ivy-Framework's `ViewContext` with its `_hooks` dictionary.
pub struct BuildContext<'a> {
    hook_index: usize,
    pub(crate) store: &'a mut HookStore,
    effects: Vec<EffectRecord>,
    /// Sender for triggering rebuilds when state changes (carries the ViewId that changed).
    rebuild_tx: Option<tokio::sync::mpsc::Sender<ViewId>>,
    /// The ViewId of the view currently being built.
    pub(crate) current_view_id: ViewId,
    event_registry: EventRegistry,
    widget_id_counter: usize,
    /// Child views registered during this build via `child_view()`.
    pub(crate) child_views: Vec<ChildViewEntry>,
    /// Cloned snapshots of ancestor context maps for safe context lookup.
    /// Each entry is a (ViewId, context map) pair, cheaply cloned via Arc.
    pub(crate) ancestor_contexts: Vec<(ViewId, ContextSnapshot)>,
}

/// Cleanup function returned by an effect callback.
pub type EffectCleanup = Box<dyn FnOnce() + Send + Sync>;

/// The boxed effect callback type (returns an optional cleanup function).
pub type EffectCallback = Box<dyn FnOnce() -> Option<EffectCleanup> + Send>;

/// An effect registered during a build, to be processed by the runtime.
pub struct EffectRecord {
    pub callback: EffectCallback,
    pub hook_index: usize,
}

/// Entry for a child view registered during build via `child_view()`.
pub struct ChildViewEntry {
    pub child_view_id: ViewId,
    pub view: Box<dyn View>,
    pub element: Element,
}

impl<'a> BuildContext<'a> {
    pub fn new(
        store: &'a mut HookStore,
        rebuild_tx: Option<tokio::sync::mpsc::Sender<ViewId>>,
    ) -> Self {
        BuildContext::with_view_id(store, rebuild_tx, uuid::Uuid::nil())
    }

    pub fn with_view_id(
        store: &'a mut HookStore,
        rebuild_tx: Option<tokio::sync::mpsc::Sender<ViewId>>,
        view_id: ViewId,
    ) -> Self {
        BuildContext {
            hook_index: 0,
            store,
            effects: Vec::new(),
            rebuild_tx,
            current_view_id: view_id,
            event_registry: EventRegistry::new(),
            widget_id_counter: 0,
            child_views: Vec::new(),
            ancestor_contexts: Vec::new(),
        }
    }

    /// Reset hook index to 0 between builds (like Ivy's ViewContext.Reset()).
    pub fn reset(&mut self) {
        self.hook_index = 0;
    }

    /// Generate the next deterministic widget ID (e.g., "w-0", "w-1", ...).
    pub fn next_widget_id(&mut self) -> String {
        let id = format!("w-{}", self.widget_id_counter);
        self.widget_id_counter += 1;
        id
    }

    /// Register an event handler for a widget.
    pub fn register_event(&mut self, widget_id: &str, event_name: &str, callback: EventCallback) {
        self.event_registry
            .register(widget_id, event_name, callback);
    }

    /// Take ownership of the event registry (called by runtime after build).
    pub fn take_event_registry(&mut self) -> EventRegistry {
        std::mem::take(&mut self.event_registry)
    }

    /// Get a mutable reference to the event registry.
    pub fn event_registry_mut(&mut self) -> &mut EventRegistry {
        &mut self.event_registry
    }

    /// Get the next hook index (for hooks to track call order).
    pub fn next_hook_index(&mut self) -> usize {
        let idx = self.hook_index;
        self.hook_index += 1;
        idx
    }

    /// Get a clone of the rebuild sender (for State to trigger re-renders).
    /// The sender carries the ViewId so the runtime knows which subtree to rebuild.
    pub fn rebuild_sender(&self) -> Option<(tokio::sync::mpsc::Sender<ViewId>, ViewId)> {
        self.rebuild_tx.clone().map(|tx| (tx, self.current_view_id))
    }

    /// Register an effect to run after build with cleanup support.
    pub fn register_effect(&mut self, hook_index: usize, callback: EffectCallback) {
        self.effects.push(EffectRecord {
            callback,
            hook_index,
        });
    }

    /// Drain all registered effects (called by runtime after build).
    pub fn drain_effects(&mut self) -> Vec<EffectRecord> {
        std::mem::take(&mut self.effects)
    }

    /// Drain child views registered during this build.
    pub fn drain_child_views(&mut self) -> Vec<ChildViewEntry> {
        std::mem::take(&mut self.child_views)
    }

    /// Compute a deterministic child ViewId from parent ViewId + child index.
    fn child_view_id(&self, child_index: usize) -> ViewId {
        let mut hasher = DefaultHasher::new();
        self.current_view_id.hash(&mut hasher);
        child_index.hash(&mut hasher);
        let hash = hasher.finish();
        let bytes = hash.to_le_bytes();
        uuid::Uuid::from_bytes([
            bytes[0],
            bytes[1],
            bytes[2],
            bytes[3],
            bytes[4],
            bytes[5],
            bytes[6],
            bytes[7],
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            (child_index & 0xFF) as u8,
        ])
    }

    /// Embed a child view within the current view's build output.
    ///
    /// Assigns a stable ViewId based on the call-site index within the parent
    /// (similar to hook ordering). The child view gets its own HookStore and
    /// is registered in the ViewTree under the current view.
    ///
    /// The `child_store` parameter is an optional pre-existing HookStore for this child.
    /// If None, a fresh store is created. Pass in the store from previous builds to
    /// preserve hook state across re-renders.
    pub fn child_view(
        &mut self,
        view: impl View,
        child_store: Option<&mut HookStore>,
    ) -> (Element, ViewId, HookStore) {
        let child_index = self.child_views.len();
        let child_view_id = self.child_view_id(child_index);

        let mut owned_store = child_store.map(std::mem::take).unwrap_or_default();

        let mut child_ctx =
            BuildContext::with_view_id(&mut owned_store, self.rebuild_tx.clone(), child_view_id);
        child_ctx.reset();
        // Set ancestor contexts: current view's store + all ancestors above.
        // Clone via Arc::clone per entry (cheap reference count bump).
        child_ctx.ancestor_contexts = self.ancestor_contexts.clone();
        child_ctx
            .ancestor_contexts
            .push((self.current_view_id, self.store.contexts.clone()));

        let mut element = view.build(&mut child_ctx);
        element.assign_ids(&mut child_ctx);

        // Merge child's event registry into parent's
        let child_registry = child_ctx.take_event_registry();
        self.event_registry.merge(child_registry);

        // Collect child effects into parent
        let child_effects = child_ctx.drain_effects();
        self.effects.extend(child_effects);

        // Collect nested child views
        let nested_children = child_ctx.drain_child_views();
        self.child_views.extend(nested_children);

        self.child_views.push(ChildViewEntry {
            child_view_id,
            view: Box::new(view),
            element: element.clone(),
        });

        (element, child_view_id, owned_store)
    }

    /// Look up a context value by TypeId, walking the ancestor chain.
    /// Returns None if not found in any ancestor.
    pub fn find_ancestor_context(&self, type_id: std::any::TypeId) -> Option<&dyn Any> {
        // First check current store
        if let Some(val) = self.store.contexts.get(&type_id) {
            return Some(val.as_ref());
        }
        // Walk ancestors from nearest to farthest (safe — no raw pointers)
        for (_view_id, contexts) in self.ancestor_contexts.iter().rev() {
            if let Some(val) = contexts.get(&type_id) {
                return Some(val.as_ref());
            }
        }
        None
    }
}

/// Implement View for closures.
impl<F> View for F
where
    F: Fn(&mut BuildContext) -> Element + Send + Sync + 'static,
{
    fn build(&self, ctx: &mut BuildContext) -> Element {
        (self)(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::widgets::button::Button;
    use crate::widgets::card::Card;
    use crate::widgets::dialog::Dialog;
    use crate::widgets::input::TextInput;
    use crate::widgets::layout::Layout;
    use crate::widgets::text::TextBlock;
    use crate::widgets::tooltip::Tooltip;
    use std::sync::Arc;

    #[test]
    fn test_assign_ids_flat_widgets() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element = Element::Fragment(vec![
            TextBlock::new("Hello").into(),
            TextBlock::new("World").into(),
            Button::new("Click").into(),
        ]);

        element.assign_ids(&mut ctx);

        // All widgets should now have IDs
        if let Element::Fragment(children) = &element {
            for (i, child) in children.iter().enumerate() {
                if let Element::Widget(w) = child {
                    assert_eq!(w.get_id(), Some(format!("w-{}", i).as_str()));
                }
            }
        }
    }

    #[test]
    fn test_assign_ids_recurses_into_fragment() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element = Element::Fragment(vec![
            Element::Fragment(vec![TextBlock::new("Nested").into()]),
            Button::new("Top").into(),
        ]);

        element.assign_ids(&mut ctx);

        if let Element::Fragment(children) = &element {
            if let Element::Fragment(nested) = &children[0] {
                if let Element::Widget(w) = &nested[0] {
                    assert_eq!(w.get_id(), Some("w-0"));
                }
            }
            if let Element::Widget(w) = &children[1] {
                assert_eq!(w.get_id(), Some("w-1"));
            }
        }
    }

    #[test]
    fn test_assign_ids_recurses_into_container_children() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = Layout::vertical()
            .child(TextBlock::new("Child 1"))
            .child(Card::new().child(TextBlock::new("Card child")))
            .into();

        element.assign_ids(&mut ctx);

        // Layout gets w-0, TextBlock child gets w-1, Card gets w-2, Card's child gets w-3
        if let Element::Widget(layout) = &element {
            assert_eq!(layout.get_id(), Some("w-0"));
        }
    }

    #[test]
    fn test_assign_ids_registers_button_click() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let clicked = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let clicked_clone = clicked.clone();

        let mut element: Element = Button::new("Click")
            .on_click(move || {
                clicked_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            })
            .into();

        element.assign_ids(&mut ctx);

        // Verify the event was registered
        let registry = ctx.take_event_registry();
        let dispatched = registry.dispatch("w-0", "click", serde_json::Value::Null);
        assert!(dispatched);
        assert!(clicked.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_assign_ids_registers_text_input_change() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received = Arc::new(std::sync::Mutex::new(String::new()));
        let received_clone = received.clone();

        let mut element: Element = TextInput::new()
            .on_change(move |val| {
                *received_clone.lock().unwrap() = val;
            })
            .into();

        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        registry.dispatch("w-0", "change", serde_json::json!({"value": "test"}));
        assert_eq!(*received.lock().unwrap(), "test");
    }

    #[test]
    fn test_assign_ids_skips_widgets_with_existing_id() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut btn = Button::new("Pre-assigned");
        btn.id = Some("custom-id".to_string());

        let mut element: Element = Element::Fragment(vec![
            Element::Widget(Box::new(btn)),
            TextBlock::new("Auto").into(),
        ]);

        element.assign_ids(&mut ctx);

        if let Element::Fragment(children) = &element {
            if let Element::Widget(w) = &children[0] {
                assert_eq!(w.get_id(), Some("custom-id")); // preserved
            }
            if let Element::Widget(w) = &children[1] {
                assert_eq!(w.get_id(), Some("w-0")); // auto-assigned
            }
        }
    }

    #[test]
    fn test_assign_ids_recurses_into_dialog_children() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = Dialog::new(true)
            .child(TextBlock::new("Dialog content"))
            .into();

        element.assign_ids(&mut ctx);

        if let Element::Widget(dialog) = &element {
            assert_eq!(dialog.get_id(), Some("w-0"));
        }
    }

    #[test]
    fn test_assign_ids_recurses_into_tooltip_child() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = Tooltip::new("Tip", Button::new("Hover me")).into();

        element.assign_ids(&mut ctx);

        if let Element::Widget(tooltip) = &element {
            assert_eq!(tooltip.get_id(), Some("w-0"));
        }
    }

    #[test]
    fn test_tooltip_child_button_click_dispatched() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let clicked = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let clicked_clone = clicked.clone();

        let mut element: Element = Tooltip::new(
            "tip",
            Button::new("Click").on_click(move || {
                clicked_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            }),
        )
        .into();

        element.assign_ids(&mut ctx);

        // Tooltip gets w-0, inner Button gets w-1
        if let Element::Widget(tooltip) = &element {
            assert_eq!(tooltip.get_id(), Some("w-0"));
        }

        // Dispatch the click event on the inner Button's ID
        let registry = ctx.take_event_registry();
        let dispatched = registry.dispatch("w-1", "click", serde_json::Value::Null);
        assert!(
            dispatched,
            "click event should be registered for inner Button w-1"
        );
        assert!(
            clicked.load(std::sync::atomic::Ordering::SeqCst),
            "on_click handler should have fired"
        );
    }
}
