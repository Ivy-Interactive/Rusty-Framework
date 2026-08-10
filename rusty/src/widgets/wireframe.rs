use crate::shared::Color;
use crate::views::view::Element;
use rusty_macros::Widget;
use serde::{Deserialize, Serialize};

/// A design annotation callout pointing at the content it wraps.
#[derive(Debug, Clone, Serialize, Deserialize, Widget)]
pub struct WireframeCallout {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    pub text: String,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[prop]
    #[children]
    pub children: Vec<Element>,
}

impl WireframeCallout {
    pub fn new(text: &str) -> Self {
        WireframeCallout {
            id: None,
            text: text.to_string(),
            title: None,
            color: None,
            children: Vec::new(),
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
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

impl From<WireframeCallout> for Element {
    fn from(callout: WireframeCallout) -> Self {
        callout.into_element()
    }
}

/// A design sticky note, optionally attributed to an author.
#[derive(Debug, Clone, Serialize, Deserialize, Widget)]
pub struct WireframeNote {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    pub text: String,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}

impl WireframeNote {
    pub fn new(text: &str) -> Self {
        WireframeNote {
            id: None,
            text: text.to_string(),
            author: None,
            color: None,
        }
    }

    pub fn author(mut self, author: &str) -> Self {
        self.author = Some(author.to_string());
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

impl From<WireframeNote> for Element {
    fn from(note: WireframeNote) -> Self {
        note.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::shared::NamedColor;
    use crate::views::view::{BuildContext, WidgetData};
    use crate::widgets::text::TextBlock;

    #[test]
    fn test_wireframe_callout_builder() {
        let callout = WireframeCallout::new("Move this button up")
            .title("UX note")
            .color(Color::Named(NamedColor::Warning))
            .child(TextBlock::new("Save"));

        assert_eq!(callout.text, "Move this button up");
        assert_eq!(callout.title.as_deref(), Some("UX note"));
        assert_eq!(callout.children.len(), 1);
    }

    #[test]
    fn test_wireframe_callout_json() {
        let json = WireframeCallout::new("Move this button up")
            .title("UX note")
            .child(TextBlock::new("Save"))
            .to_json();

        assert_eq!(json["type"], "wireframe_callout");
        assert_eq!(json["text"], "Move this button up");
        assert_eq!(json["title"], "UX note");
        assert_eq!(json["children"][0]["content"], "Save");
    }

    #[test]
    fn test_wireframe_callout_assign_ids_recurses_into_children() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = WireframeCallout::new("Note")
            .child(TextBlock::new("Child"))
            .into();
        element.assign_ids(&mut ctx);

        if let Element::Widget(ref w) = element {
            let json = w.to_json();
            assert_eq!(json["id"], "w-0");
            assert_eq!(json["children"][0]["id"], "w-1");
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_wireframe_note_builder() {
        let note = WireframeNote::new("Consider dark mode").author("Alex");

        assert_eq!(note.text, "Consider dark mode");
        assert_eq!(note.author.as_deref(), Some("Alex"));
    }

    #[test]
    fn test_wireframe_note_json() {
        let json = WireframeNote::new("Consider dark mode")
            .author("Alex")
            .to_json();

        assert_eq!(json["type"], "wireframe_note");
        assert_eq!(json["text"], "Consider dark mode");
        assert_eq!(json["author"], "Alex");
    }
}
