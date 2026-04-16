use crate::shared::{Align, Justify};
use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LayoutDirection {
    Vertical,
    Horizontal,
    Grid,
}

/// A flexbox-style layout container widget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    pub direction: LayoutDirection,
    pub children: Vec<Element>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justify: Option<Justify>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<usize>,
}

impl Layout {
    pub fn vertical() -> Self {
        Layout {
            direction: LayoutDirection::Vertical,
            children: Vec::new(),
            gap: None,
            align: None,
            justify: None,
            padding: None,
            columns: None,
        }
    }

    pub fn horizontal() -> Self {
        Layout {
            direction: LayoutDirection::Horizontal,
            children: Vec::new(),
            gap: None,
            align: None,
            justify: None,
            padding: None,
            columns: None,
        }
    }

    pub fn grid(columns: usize) -> Self {
        Layout {
            direction: LayoutDirection::Grid,
            children: Vec::new(),
            gap: None,
            align: None,
            justify: None,
            padding: None,
            columns: Some(columns),
        }
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

impl WidgetData for Layout {
    fn widget_type(&self) -> &str {
        "layout"
    }

    fn to_json(&self) -> serde_json::Value {
        let children_json: Vec<serde_json::Value> = self
            .children
            .iter()
            .map(|c| serde_json::to_value(c).unwrap_or_default())
            .collect();

        json!({
            "type": "layout",
            "direction": self.direction,
            "children": children_json,
            "gap": self.gap,
            "align": self.align,
            "justify": self.justify,
            "padding": self.padding,
            "columns": self.columns,
        })
    }

    fn clone_box(&self) -> Box<dyn WidgetData> {
        Box::new(self.clone())
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
}
