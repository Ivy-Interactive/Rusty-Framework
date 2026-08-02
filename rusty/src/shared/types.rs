use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Unique identifier for views and widgets.
pub type ViewId = uuid::Uuid;
pub type WidgetId = uuid::Uuid;

/// Size specification for widgets.
///
/// Serializes to a CSS length string (`"8px"`, `"50%"`, `"auto"`) and
/// deserializes back from one, so `Serialize` and [`Size::to_css`] agree and the
/// value round-trips. A `#[serde(untagged)]` derive would collapse `Px(8.0)` and
/// `Percent(8.0)` to a bare `8.0` and `Auto` to `null`.
#[derive(Debug, Clone, PartialEq)]
pub enum Size {
    Px(f64),
    Percent(f64),
    Auto,
}

impl Size {
    /// Render as a CSS length. This is the wire form: `Serialize` emits exactly
    /// this string.
    pub fn to_css(&self) -> String {
        match self {
            Size::Px(px) => format!("{}px", px),
            Size::Percent(pct) => format!("{}%", pct),
            Size::Auto => "auto".to_string(),
        }
    }

    /// Parse a CSS length string produced by [`Size::to_css`].
    pub fn parse_css(value: &str) -> Option<Size> {
        let trimmed = value.trim();
        if trimmed.eq_ignore_ascii_case("auto") {
            return Some(Size::Auto);
        }
        if let Some(num) = trimmed.strip_suffix('%') {
            return num.trim().parse::<f64>().ok().map(Size::Percent);
        }
        if let Some(num) = trimmed.strip_suffix("px") {
            return num.trim().parse::<f64>().ok().map(Size::Px);
        }
        None
    }
}

impl Serialize for Size {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_css())
    }
}

impl<'de> Deserialize<'de> for Size {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SizeVisitor;

        impl Visitor<'_> for SizeVisitor {
            type Value = Size;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a CSS length such as \"8px\", \"50%\" or \"auto\"")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Size, E> {
                Size::parse_css(value)
                    .ok_or_else(|| de::Error::invalid_value(de::Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_str(SizeVisitor)
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
    fn test_size_serialize_matches_to_css() {
        assert_eq!(serde_json::to_string(&Size::Px(8.0)).unwrap(), "\"8px\"");
        assert_eq!(
            serde_json::to_string(&Size::Percent(8.0)).unwrap(),
            "\"8%\""
        );
        assert_eq!(serde_json::to_string(&Size::Auto).unwrap(), "\"auto\"");
    }

    #[test]
    fn test_size_round_trips() {
        let px = Size::Px(8.0);
        let json = serde_json::to_string(&px).unwrap();
        assert_eq!(serde_json::from_str::<Size>(&json).unwrap(), px);

        let percent = Size::Percent(50.0);
        let json = serde_json::to_string(&percent).unwrap();
        assert_eq!(serde_json::from_str::<Size>(&json).unwrap(), percent);

        let auto = Size::Auto;
        let json = serde_json::to_string(&auto).unwrap();
        assert_eq!(serde_json::from_str::<Size>(&json).unwrap(), auto);
    }

    #[test]
    fn test_size_deserialize_rejects_bare_number_and_unknown_unit() {
        assert!(serde_json::from_str::<Size>("8.0").is_err());
        assert!(serde_json::from_str::<Size>("null").is_err());
        assert!(serde_json::from_str::<Size>("\"8em\"").is_err());
    }

    #[test]
    fn test_size_option_none_is_null() {
        let none: Option<Size> = None;
        assert_eq!(serde_json::to_string(&none).unwrap(), "null");
    }
}
