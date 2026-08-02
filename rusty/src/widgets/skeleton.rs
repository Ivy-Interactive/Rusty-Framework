use crate::shared::Size;
use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// A placeholder block shown while content loads.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Skeleton {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Size>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<Size>,
}

impl Skeleton {
    pub fn new() -> Self {
        Skeleton::default()
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

impl WidgetData for Skeleton {
    fn widget_type(&self) -> &str {
        "skeleton"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "skeleton",
            "id": self.id,
            "width": self.width,
            "height": self.height,
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

impl From<Skeleton> for Element {
    fn from(skeleton: Skeleton) -> Self {
        skeleton.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skeleton_builder() {
        let skeleton = Skeleton::new()
            .width(Size::Percent(80.0))
            .height(Size::Px(16.0));
        assert_eq!(skeleton.width, Some(Size::Percent(80.0)));
        assert_eq!(skeleton.height, Some(Size::Px(16.0)));
    }

    #[test]
    fn test_skeleton_json() {
        let json = Skeleton::new().width(Size::Px(200.0)).to_json();
        assert_eq!(json["type"], "skeleton");
        assert_eq!(json["width"], "200px");
        assert!(json["height"].is_null());
    }
}
