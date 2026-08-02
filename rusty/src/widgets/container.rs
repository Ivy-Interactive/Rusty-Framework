use crate::shared::{Color, Size};
use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// A styled box wrapping arbitrary children.
///
/// Named `Container` rather than `Box`: `rusty::prelude::*` is glob-imported by
/// every binary and example, all of which also use `Box::new`, so a `Box`
/// widget in the prelude would shadow `std::boxed::Box`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Container {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub children: Vec<Element>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Size>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<Size>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Color>,
    pub border: bool,
    pub rounded: bool,
}

impl Container {
    pub fn new() -> Self {
        Container::default()
    }

    pub fn child(mut self, element: impl Into<Element>) -> Self {
        self.children.push(element.into());
        self
    }

    pub fn children(mut self, elements: Vec<Element>) -> Self {
        self.children.extend(elements);
        self
    }

    pub fn padding(mut self, padding: f64) -> Self {
        self.padding = Some(padding);
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

    pub fn background(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }

    pub fn border(mut self, border: bool) -> Self {
        self.border = border;
        self
    }

    pub fn rounded(mut self, rounded: bool) -> Self {
        self.rounded = rounded;
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for Container {
    fn widget_type(&self) -> &str {
        "container"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "container",
            "id": self.id,
            "children": self.children.iter()
                .map(|c| serde_json::to_value(c).unwrap_or_default())
                .collect::<Vec<_>>(),
            "padding": self.padding,
            "width": self.width,
            "height": self.height,
            "background": self.background,
            "border": self.border,
            "rounded": self.rounded,
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

    fn children_mut(&mut self) -> Option<&mut Vec<Element>> {
        Some(&mut self.children)
    }
}

impl From<Container> for Element {
    fn from(container: Container) -> Self {
        container.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::shared::NamedColor;
    use crate::views::view::BuildContext;
    use crate::widgets::text::TextBlock;

    #[test]
    fn test_container_builder() {
        let container = Container::new()
            .padding(12.0)
            .width(Size::Percent(50.0))
            .height(Size::Px(200.0))
            .background(Color::Named(NamedColor::Muted))
            .border(true)
            .rounded(true)
            .child(TextBlock::new("Inside"));

        assert_eq!(container.padding, Some(12.0));
        assert_eq!(container.width, Some(Size::Percent(50.0)));
        assert_eq!(container.height, Some(Size::Px(200.0)));
        assert_eq!(container.background, Some(Color::Named(NamedColor::Muted)));
        assert!(container.border);
        assert!(container.rounded);
        assert_eq!(container.children.len(), 1);
    }

    #[test]
    fn test_container_json() {
        let json = Container::new()
            .padding(8.0)
            .width(Size::Auto)
            .border(true)
            .child(TextBlock::new("Body"))
            .to_json();

        assert_eq!(json["type"], "container");
        assert_eq!(json["padding"], 8.0);
        assert_eq!(json["width"], "auto");
        assert_eq!(json["border"], true);
        assert_eq!(json["rounded"], false);
        assert_eq!(json["children"][0]["content"], "Body");
    }

    #[test]
    fn test_container_assign_ids_recurses_into_children() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = Container::new()
            .child(TextBlock::new("First"))
            .child(TextBlock::new("Second"))
            .into();

        element.assign_ids(&mut ctx);

        if let Element::Widget(ref w) = element {
            let json = w.to_json();
            assert_eq!(json["id"], "w-0");
            assert_eq!(json["children"][0]["id"], "w-1");
            assert_eq!(json["children"][1]["id"], "w-2");
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_container_children_vec_builder() {
        let container = Container::new().children(vec![
            TextBlock::new("One").into(),
            TextBlock::new("Two").into(),
        ]);
        assert_eq!(container.children.len(), 2);
    }
}
