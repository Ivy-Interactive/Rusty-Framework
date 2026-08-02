use crate::shared::{Color, Icon};
use crate::views::view::Element;
use rusty_macros::Widget;
use serde::{Deserialize, Serialize};

/// A renderable icon.
///
/// Named `IconWidget` rather than `Icon` to avoid colliding with
/// [`crate::shared::Icon`], the value type already in the prelude, which this
/// widget wraps. The wire name stays `"icon"` — that is what the frontend
/// registry keys on — hence the `#[widget(type = ...)]` override.
#[derive(Debug, Clone, Serialize, Deserialize, Widget)]
#[widget(type = "icon")]
pub struct IconWidget {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    pub name: Icon,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}

impl IconWidget {
    pub fn new(name: impl Into<Icon>) -> Self {
        IconWidget {
            id: None,
            name: name.into(),
            size: None,
            color: None,
        }
    }

    pub fn size(mut self, size: f64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl From<IconWidget> for Element {
    fn from(icon: IconWidget) -> Self {
        icon.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::NamedColor;
    use crate::views::view::WidgetData;

    #[test]
    fn test_icon_widget_builder() {
        let icon = IconWidget::new("check")
            .size(24.0)
            .color(Color::Named(NamedColor::Success));

        assert_eq!(icon.name, Icon::new("check"));
        assert_eq!(icon.size, Some(24.0));
        assert_eq!(icon.color, Some(Color::Named(NamedColor::Success)));
    }

    #[test]
    fn test_icon_widget_json() {
        let json = IconWidget::new("trash").size(16.0).to_json();
        assert_eq!(json["type"], "icon");
        assert_eq!(json["name"], "trash");
        assert_eq!(json["size"], 16.0);
        assert!(json["color"].is_null());
    }

    #[test]
    fn test_icon_widget_accepts_icon_value() {
        let icon = IconWidget::new(Icon::new("star"));
        assert_eq!(icon.to_json()["name"], "star");
    }
}
