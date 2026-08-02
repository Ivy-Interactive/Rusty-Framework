use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A color value supporting named colors, hex, and RGBA.
///
/// Serializes to a CSS color string (`"primary"`, `"#ff0000"`, `"rgba(1, 2, 3, 0.5)"`) and
/// deserializes back from one, so `Serialize` and [`Color::to_css`] agree and the
/// value round-trips. A `#[serde(untagged)]` derive would emit named/hex as strings
/// but rgba as an object `{"r":1,"g":2,"b":3,"a":0.5}`, forcing clients to sniff the type.
#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    Named(NamedColor),
    Hex(String),
    Rgba { r: u8, g: u8, b: u8, a: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NamedColor {
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
    Info,
    Muted,
    White,
    Black,
}

impl NamedColor {
    /// The camelCase wire name, matching the `rename_all` derive.
    pub fn as_str(&self) -> &'static str {
        match self {
            NamedColor::Primary => "primary",
            NamedColor::Secondary => "secondary",
            NamedColor::Success => "success",
            NamedColor::Warning => "warning",
            NamedColor::Danger => "danger",
            NamedColor::Info => "info",
            NamedColor::Muted => "muted",
            NamedColor::White => "white",
            NamedColor::Black => "black",
        }
    }

    /// Parse a camelCase wire name back into a variant.
    pub fn parse(value: &str) -> Option<NamedColor> {
        match value {
            "primary" => Some(NamedColor::Primary),
            "secondary" => Some(NamedColor::Secondary),
            "success" => Some(NamedColor::Success),
            "warning" => Some(NamedColor::Warning),
            "danger" => Some(NamedColor::Danger),
            "info" => Some(NamedColor::Info),
            "muted" => Some(NamedColor::Muted),
            "white" => Some(NamedColor::White),
            "black" => Some(NamedColor::Black),
            _ => None,
        }
    }
}

impl Color {
    pub fn hex(value: &str) -> Self {
        Color::Hex(value.to_string())
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Color::Rgba { r, g, b, a }
    }

    /// Render as a CSS color. This is the wire form: `Serialize` emits exactly
    /// this string.
    pub fn to_css(&self) -> String {
        match self {
            Color::Named(named) => named.as_str().to_string(),
            Color::Hex(hex) => hex.clone(),
            Color::Rgba { r, g, b, a } => format!("rgba({}, {}, {}, {})", r, g, b, a),
        }
    }

    /// Parse a CSS color string produced by [`Color::to_css`].
    pub fn parse_css(value: &str) -> Option<Color> {
        let trimmed = value.trim();
        if let Some(named) = NamedColor::parse(trimmed) {
            return Some(Color::Named(named));
        }
        if trimmed.starts_with('#') {
            return Some(Color::Hex(trimmed.to_string()));
        }
        if let Some(args) = trimmed
            .strip_prefix("rgba(")
            .and_then(|rest| rest.strip_suffix(')'))
        {
            let parts: Vec<&str> = args.split(',').map(str::trim).collect();
            if parts.len() == 4 {
                return Some(Color::Rgba {
                    r: parts[0].parse().ok()?,
                    g: parts[1].parse().ok()?,
                    b: parts[2].parse().ok()?,
                    a: parts[3].parse().ok()?,
                });
            }
        }
        None
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_css())
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ColorVisitor;

        impl Visitor<'_> for ColorVisitor {
            type Value = Color;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(
                    "a CSS color such as \"primary\", \"#ff0000\" or \"rgba(1, 2, 3, 0.5)\"",
                )
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Color, E> {
                Color::parse_css(value)
                    .ok_or_else(|| de::Error::invalid_value(de::Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_str(ColorVisitor)
    }
}

impl From<NamedColor> for Color {
    fn from(named: NamedColor) -> Self {
        Color::Named(named)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_serialization() {
        let color = Color::Named(NamedColor::Primary);
        let json = serde_json::to_string(&color).unwrap();
        assert!(json.contains("primary") || json.contains("Primary"));
    }

    #[test]
    fn test_hex_color() {
        let color = Color::hex("#ff0000");
        if let Color::Hex(val) = color {
            assert_eq!(val, "#ff0000");
        } else {
            panic!("Expected hex color");
        }
    }

    #[test]
    fn test_color_serialize_matches_to_css() {
        assert_eq!(
            serde_json::to_string(&Color::Named(NamedColor::Primary)).unwrap(),
            "\"primary\""
        );
        assert_eq!(
            serde_json::to_string(&Color::Hex("#ff0000".to_string())).unwrap(),
            "\"#ff0000\""
        );
        assert_eq!(
            serde_json::to_string(&Color::Rgba {
                r: 1,
                g: 2,
                b: 3,
                a: 0.5
            })
            .unwrap(),
            "\"rgba(1, 2, 3, 0.5)\""
        );
    }

    #[test]
    fn test_color_round_trips() {
        let named = Color::Named(NamedColor::Primary);
        let json = serde_json::to_string(&named).unwrap();
        assert_eq!(serde_json::from_str::<Color>(&json).unwrap(), named);

        let hex = Color::Hex("#ff0000".to_string());
        let json = serde_json::to_string(&hex).unwrap();
        assert_eq!(serde_json::from_str::<Color>(&json).unwrap(), hex);

        let rgba = Color::Rgba {
            r: 1,
            g: 2,
            b: 3,
            a: 0.5,
        };
        let json = serde_json::to_string(&rgba).unwrap();
        assert_eq!(serde_json::from_str::<Color>(&json).unwrap(), rgba);
    }

    #[test]
    fn test_color_deserialize_rejects_object_and_garbage() {
        assert!(serde_json::from_str::<Color>("{\"r\":1,\"g\":2,\"b\":3,\"a\":1.0}").is_err());
        assert!(serde_json::from_str::<Color>("\"notacolor\"").is_err());
    }
}
