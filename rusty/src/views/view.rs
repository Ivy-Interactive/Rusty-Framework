use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;

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

/// Context passed to View::build providing access to hooks and state.
pub struct BuildContext {
    hook_index: usize,
    states: Vec<Box<dyn Any + Send + Sync>>,
    effects: Vec<Box<dyn FnOnce() + Send>>,
}

impl BuildContext {
    pub fn new() -> Self {
        BuildContext {
            hook_index: 0,
            states: Vec::new(),
            effects: Vec::new(),
        }
    }

    /// Get the next hook index (for hooks to track call order).
    pub fn next_hook_index(&mut self) -> usize {
        let idx = self.hook_index;
        self.hook_index += 1;
        idx
    }

    /// Store a state value for a hook.
    pub fn store_state(&mut self, state: Box<dyn Any + Send + Sync>) {
        self.states.push(state);
    }

    /// Register an effect to run after build.
    pub fn register_effect(&mut self, effect: Box<dyn FnOnce() + Send>) {
        self.effects.push(effect);
    }

    /// Drain all registered effects (called by runtime after build).
    pub fn drain_effects(&mut self) -> Vec<Box<dyn FnOnce() + Send>> {
        std::mem::take(&mut self.effects)
    }
}

impl Default for BuildContext {
    fn default() -> Self {
        Self::new()
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
