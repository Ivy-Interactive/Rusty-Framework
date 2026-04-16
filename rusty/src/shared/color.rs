use serde::{Deserialize, Serialize};

/// A color value supporting named colors, hex, and RGBA.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
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

impl Color {
    pub fn hex(value: &str) -> Self {
        Color::Hex(value.to_string())
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Color::Rgba { r, g, b, a }
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
}
