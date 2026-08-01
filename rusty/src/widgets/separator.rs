use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Axis along which a widget lays out its content.
///
/// Introduced for `Separator` and reused by `RadioGroup`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

/// A dividing rule, optionally labelled with inline text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Separator {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub orientation: Orientation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Separator {
    pub fn horizontal() -> Self {
        Separator {
            id: None,
            orientation: Orientation::Horizontal,
            text: None,
        }
    }

    pub fn vertical() -> Self {
        Separator {
            id: None,
            orientation: Orientation::Vertical,
            text: None,
        }
    }

    pub fn text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for Separator {
    fn default() -> Self {
        Self::horizontal()
    }
}

impl WidgetData for Separator {
    fn widget_type(&self) -> &str {
        "separator"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "separator",
            "id": self.id,
            "orientation": self.orientation,
            "text": self.text,
        })
    }

    fn clone_box(&self) -> Box<dyn WidgetData> {
        Box::new(self.clone())
    }

    fn assign_id(&mut self, id: String) {
        self.id = Some(id);
    }

    fn get_id(&self) -> Option<&str> {
        self.id.as_deref()
    }
}

impl From<Separator> for Element {
    fn from(separator: Separator) -> Self {
        separator.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_separator_builders() {
        assert_eq!(Separator::horizontal().orientation, Orientation::Horizontal);
        assert_eq!(Separator::vertical().orientation, Orientation::Vertical);
        assert_eq!(
            Separator::horizontal().text("OR").text.as_deref(),
            Some("OR")
        );
    }

    #[test]
    fn test_separator_json() {
        let json = Separator::vertical().text("OR").to_json();
        assert_eq!(json["type"], "separator");
        assert_eq!(json["orientation"], "vertical");
        assert_eq!(json["text"], "OR");
    }

    #[test]
    fn test_separator_json_without_text() {
        let json = Separator::horizontal().to_json();
        assert_eq!(json["orientation"], "horizontal");
        assert!(json["text"].is_null());
    }

    #[test]
    fn test_orientation_default_is_horizontal() {
        assert_eq!(Orientation::default(), Orientation::Horizontal);
    }
}
