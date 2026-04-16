use crate::shared::Color;
use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextVariant {
    Block,
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Paragraph,
    Code,
    Markdown,
    Label,
    Caption,
}

/// A text rendering widget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    pub content: String,
    pub variant: TextVariant,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    pub bold: bool,
    pub italic: bool,
}

impl TextBlock {
    pub fn new(content: &str) -> Self {
        TextBlock {
            content: content.to_string(),
            variant: TextVariant::Block,
            color: None,
            bold: false,
            italic: false,
        }
    }

    pub fn h1(content: &str) -> Self {
        Self::new(content).variant(TextVariant::Heading1)
    }

    pub fn h2(content: &str) -> Self {
        Self::new(content).variant(TextVariant::Heading2)
    }

    pub fn h3(content: &str) -> Self {
        Self::new(content).variant(TextVariant::Heading3)
    }

    pub fn paragraph(content: &str) -> Self {
        Self::new(content).variant(TextVariant::Paragraph)
    }

    pub fn code(content: &str) -> Self {
        Self::new(content).variant(TextVariant::Code)
    }

    pub fn markdown(content: &str) -> Self {
        Self::new(content).variant(TextVariant::Markdown)
    }

    pub fn label(content: &str) -> Self {
        Self::new(content).variant(TextVariant::Label)
    }

    pub fn variant(mut self, variant: TextVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for TextBlock {
    fn widget_type(&self) -> &str {
        "text_block"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "text_block",
            "content": self.content,
            "variant": self.variant,
            "color": self.color,
            "bold": self.bold,
            "italic": self.italic,
        })
    }

    fn clone_box(&self) -> Box<dyn WidgetData> {
        Box::new(self.clone())
    }
}

impl From<TextBlock> for Element {
    fn from(text: TextBlock) -> Self {
        text.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_block_new() {
        let text = TextBlock::new("Hello");
        assert_eq!(text.content, "Hello");
        assert_eq!(text.variant, TextVariant::Block);
    }

    #[test]
    fn test_text_heading() {
        let h1 = TextBlock::h1("Title");
        assert_eq!(h1.variant, TextVariant::Heading1);
    }

    #[test]
    fn test_text_serialization() {
        let text = TextBlock::paragraph("Test content");
        let json = text.to_json();
        assert_eq!(json["type"], "text_block");
        assert_eq!(json["content"], "Test content");
    }
}
