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
