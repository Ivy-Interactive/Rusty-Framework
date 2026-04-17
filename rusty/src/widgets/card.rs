use crate::views::view::{BuildContext, Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// A container widget with optional header, body, and footer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    pub children: Vec<Element>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<Vec<Element>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<f64>,
}

impl Card {
    pub fn new() -> Self {
        Card {
            id: None,
            title: None,
            subtitle: None,
            children: Vec::new(),
            footer: None,
            padding: None,
        }
    }

    /// Assign a widget ID from the BuildContext.
    #[deprecated(note = "Widget IDs are now assigned automatically. Remove .build(ctx) calls.")]
    pub fn build(mut self, ctx: &mut BuildContext) -> Self {
        self.id = Some(ctx.next_widget_id());
        self
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn subtitle(mut self, subtitle: &str) -> Self {
        self.subtitle = Some(subtitle.to_string());
        self
    }

    pub fn child(mut self, element: impl Into<Element>) -> Self {
        self.children.push(element.into());
        self
    }

    pub fn footer(mut self, elements: Vec<Element>) -> Self {
        self.footer = Some(elements);
        self
    }

    pub fn padding(mut self, padding: f64) -> Self {
        self.padding = Some(padding);
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetData for Card {
    fn widget_type(&self) -> &str {
        "card"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "card",
            "id": self.id,
            "title": self.title,
            "subtitle": self.subtitle,
            "children": self.children.iter()
                .map(|c| serde_json::to_value(c).unwrap_or_default())
                .collect::<Vec<_>>(),
            "footer": self.footer.as_ref().map(|f| f.iter()
                .map(|c| serde_json::to_value(c).unwrap_or_default())
                .collect::<Vec<_>>()),
            "padding": self.padding,
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

    fn footer_mut(&mut self) -> Option<&mut Vec<Element>> {
        self.footer.as_mut()
    }
}

impl From<Card> for Element {
    fn from(card: Card) -> Self {
        card.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_builder() {
        let card = Card::new().title("My Card").subtitle("Description");
        assert_eq!(card.title.as_deref(), Some("My Card"));
        assert_eq!(card.subtitle.as_deref(), Some("Description"));
    }
}
