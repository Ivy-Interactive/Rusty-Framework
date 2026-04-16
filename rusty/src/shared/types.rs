use serde::{Deserialize, Serialize};

/// Unique identifier for views and widgets.
pub type ViewId = uuid::Uuid;
pub type WidgetId = uuid::Uuid;

/// Size specification for widgets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Size {
    Px(f64),
    Percent(f64),
    Auto,
}

/// Density level for widget rendering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Density {
    Compact,
    #[default]
    Normal,
    Comfortable,
}

/// Alignment options for layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

/// Justify options for layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Justify {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}
