use crate::views::view::{BuildContext, Element};
use rusty_macros::Widget;
use serde::{Deserialize, Serialize};

/// A container widget with optional header, body, and footer.
#[derive(Debug, Clone, Serialize, Deserialize, Widget)]
pub struct Card {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[prop]
    #[children]
    pub children: Vec<Element>,
    #[prop]
    #[footer]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<Vec<Element>>,
    #[prop]
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
