use crate::shared::Density;
use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// A user avatar, falling back to initials when no image is available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Avatar {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    pub fallback: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<Density>,
}

impl Avatar {
    /// Create an avatar showing `fallback` (typically initials) until an image
    /// is supplied with [`Avatar::image`].
    pub fn new(fallback: &str) -> Self {
        Avatar {
            id: None,
            image: None,
            fallback: fallback.to_string(),
            size: None,
        }
    }

    pub fn image(mut self, image: &str) -> Self {
        self.image = Some(image.to_string());
        self
    }

    pub fn size(mut self, size: Density) -> Self {
        self.size = Some(size);
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for Avatar {
    fn widget_type(&self) -> &str {
        "avatar"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "avatar",
            "id": self.id,
            "image": self.image,
            "fallback": self.fallback,
            "size": self.size,
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

impl From<Avatar> for Element {
    fn from(avatar: Avatar) -> Self {
        avatar.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avatar_builder() {
        let avatar = Avatar::new("MR")
            .image("/me.png")
            .size(Density::Comfortable);

        assert_eq!(avatar.fallback, "MR");
        assert_eq!(avatar.image.as_deref(), Some("/me.png"));
        assert_eq!(avatar.size, Some(Density::Comfortable));
    }

    #[test]
    fn test_avatar_json() {
        let json = Avatar::new("AB").size(Density::Compact).to_json();
        assert_eq!(json["type"], "avatar");
        assert_eq!(json["fallback"], "AB");
        assert_eq!(json["size"], "compact");
        assert!(json["image"].is_null());
    }
}
