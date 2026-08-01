use crate::shared::Size;
use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// An image loaded from a URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub src: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Size>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<Size>,
}

impl Image {
    pub fn new(src: &str) -> Self {
        Image {
            id: None,
            src: src.to_string(),
            alt: None,
            width: None,
            height: None,
        }
    }

    pub fn alt(mut self, alt: &str) -> Self {
        self.alt = Some(alt.to_string());
        self
    }

    pub fn width(mut self, width: Size) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: Size) -> Self {
        self.height = Some(height);
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for Image {
    fn widget_type(&self) -> &str {
        "image"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "image",
            "id": self.id,
            "src": self.src,
            "alt": self.alt,
            "width": self.width.as_ref().map(Size::to_css),
            "height": self.height.as_ref().map(Size::to_css),
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

impl From<Image> for Element {
    fn from(image: Image) -> Self {
        image.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_builder() {
        let image = Image::new("/logo.png")
            .alt("Logo")
            .width(Size::Px(120.0))
            .height(Size::Auto);

        assert_eq!(image.src, "/logo.png");
        assert_eq!(image.alt.as_deref(), Some("Logo"));
        assert_eq!(image.width, Some(Size::Px(120.0)));
        assert_eq!(image.height, Some(Size::Auto));
    }

    #[test]
    fn test_image_json() {
        let json = Image::new("/a.png")
            .alt("A")
            .width(Size::Percent(100.0))
            .to_json();

        assert_eq!(json["type"], "image");
        assert_eq!(json["src"], "/a.png");
        assert_eq!(json["alt"], "A");
        assert_eq!(json["width"], "100%");
        assert!(json["height"].is_null());
    }
}
