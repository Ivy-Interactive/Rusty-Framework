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

impl Size {
    /// Render as a CSS length. Widgets serialize sizes through this rather than
    /// through `Serialize`: the derive is `untagged`, so `Px(8.0)` and
    /// `Percent(8.0)` both emit a bare `8.0` and `Auto` emits `null`, leaving a
    /// client unable to tell pixels from percent or `Auto` from unset.
    pub fn to_css(&self) -> String {
        match self {
            Size::Px(px) => format!("{}px", px),
            Size::Percent(pct) => format!("{}%", pct),
            Size::Auto => "auto".to_string(),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_to_css() {
        assert_eq!(Size::Px(8.0).to_css(), "8px");
        assert_eq!(Size::Percent(50.0).to_css(), "50%");
        assert_eq!(Size::Auto.to_css(), "auto");
    }

    #[test]
    fn test_size_serialize_is_lossy_so_widgets_use_to_css() {
        // Documents why `to_css` exists: `untagged` collapses the variants.
        assert_eq!(serde_json::to_string(&Size::Px(8.0)).unwrap(), "8.0");
        assert_eq!(serde_json::to_string(&Size::Percent(8.0)).unwrap(), "8.0");
        assert_eq!(serde_json::to_string(&Size::Auto).unwrap(), "null");
    }
}
