use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;

use crate::core::event_registry::{EventCallback, EventRegistry};
use crate::hooks::hook_store::HookStore;

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
    /// Sender for triggering rebuilds when state changes.
    rebuild_tx: Option<tokio::sync::mpsc::Sender<()>>,
    event_registry: EventRegistry,
    widget_id_counter: usize,
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

impl<'a> BuildContext<'a> {
    pub fn new(
        store: &'a mut HookStore,
        rebuild_tx: Option<tokio::sync::mpsc::Sender<()>>,
    ) -> Self {
        BuildContext {
            hook_index: 0,
            store,
            effects: Vec::new(),
            rebuild_tx,
            event_registry: EventRegistry::new(),
            widget_id_counter: 0,
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
    pub fn rebuild_sender(&self) -> Option<tokio::sync::mpsc::Sender<()>> {
        self.rebuild_tx.clone()
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
}
