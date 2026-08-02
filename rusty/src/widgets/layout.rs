use crate::shared::{Align, Justify, Size};
use crate::views::view::{BuildContext, Element};
use rusty_macros::Widget;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LayoutDirection {
    Vertical,
    Horizontal,
    Grid,
}

/// A flexbox-style layout container widget.
#[derive(Debug, Clone, Serialize, Deserialize, Widget)]
pub struct Layout {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    pub direction: LayoutDirection,
    #[prop]
    #[children]
    pub children: Vec<Element>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<f64>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justify: Option<Justify>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<f64>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<usize>,
    #[prop(with = "crate::shared::size_css")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Size>,
    #[prop(with = "crate::shared::size_css")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<Size>,
    #[prop]
    pub wrap: bool,
}

impl Layout {
    pub fn vertical() -> Self {
        Layout::with_direction(LayoutDirection::Vertical, None)
    }

    pub fn horizontal() -> Self {
        Layout::with_direction(LayoutDirection::Horizontal, None)
    }

    pub fn grid(columns: usize) -> Self {
        Layout::with_direction(LayoutDirection::Grid, Some(columns))
    }

    fn with_direction(direction: LayoutDirection, columns: Option<usize>) -> Self {
        Layout {
            id: None,
            direction,
            children: Vec::new(),
            gap: None,
            align: None,
            justify: None,
            padding: None,
            columns,
            width: None,
            height: None,
            wrap: false,
        }
    }

    /// Assign a widget ID from the BuildContext.
    #[deprecated(note = "Widget IDs are now assigned automatically. Remove .build(ctx) calls.")]
    pub fn build(mut self, ctx: &mut BuildContext) -> Self {
        self.id = Some(ctx.next_widget_id());
        self
    }

    pub fn gap(mut self, gap: f64) -> Self {
        self.gap = Some(gap);
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = Some(align);
        self
    }

    pub fn justify(mut self, justify: Justify) -> Self {
        self.justify = Some(justify);
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

    /// Allow children to wrap onto additional lines when they overflow.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn child(mut self, element: impl Into<Element>) -> Self {
        self.children.push(element.into());
        self
    }

    pub fn children(mut self, elements: Vec<Element>) -> Self {
        self.children.extend(elements);
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl From<Layout> for Element {
    fn from(layout: Layout) -> Self {
        layout.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::view::WidgetData;
    use crate::widgets::text::TextBlock;

    #[test]
    fn test_vertical_layout() {
        let layout = Layout::vertical()
            .gap(8.0)
            .child(TextBlock::new("Item 1"))
            .child(TextBlock::new("Item 2"));

        assert_eq!(layout.direction, LayoutDirection::Vertical);
        assert_eq!(layout.children.len(), 2);
        assert_eq!(layout.gap, Some(8.0));
    }

    #[test]
    fn test_grid_layout() {
        let layout = Layout::grid(3);
        assert_eq!(layout.direction, LayoutDirection::Grid);
        assert_eq!(layout.columns, Some(3));
    }

    #[test]
    fn test_layout_sizing_builders() {
        let layout = Layout::horizontal()
            .width(Size::Percent(100.0))
            .height(Size::Px(240.0))
            .wrap(true);

        assert_eq!(layout.width, Some(Size::Percent(100.0)));
        assert_eq!(layout.height, Some(Size::Px(240.0)));
        assert!(layout.wrap);
    }

    #[test]
    fn test_layout_json_includes_sizing() {
        let json = Layout::vertical()
            .width(Size::Auto)
            .height(Size::Px(64.0))
            .wrap(true)
            .to_json();

        assert_eq!(json["type"], "layout");
        assert_eq!(json["width"], "auto");
        assert_eq!(json["height"], "64px");
        assert_eq!(json["wrap"], true);
    }

    #[test]
    fn test_layout_json_omits_unset_sizing() {
        let json = Layout::vertical().to_json();
        assert!(json["width"].is_null());
        assert!(json["height"].is_null());
        assert_eq!(json["wrap"], false);
    }
}
