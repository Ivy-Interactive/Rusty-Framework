use crate::shared::Color;
use crate::views::view::{BuildContext, Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// A progress bar widget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    pub indeterminate: bool,
}

impl Progress {
    pub fn new(value: f64) -> Self {
        Progress {
            id: None,
            value,
            max: None,
            label: None,
            color: None,
            indeterminate: false,
        }
    }

    pub fn indeterminate() -> Self {
        Progress {
            id: None,
            value: 0.0,
            max: None,
            label: None,
            color: None,
            indeterminate: true,
        }
    }

    /// Assign a widget ID from the BuildContext.
    #[deprecated(note = "Widget IDs are now assigned automatically. Remove .build(ctx) calls.")]
    pub fn build(mut self, ctx: &mut BuildContext) -> Self {
        self.id = Some(ctx.next_widget_id());
        self
    }

    pub fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
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

impl WidgetData for Progress {
    fn widget_type(&self) -> &str {
        "progress"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "progress",
            "id": self.id,
            "value": self.value,
            "max": self.max,
            "label": self.label,
            "color": self.color,
            "indeterminate": self.indeterminate,
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

impl From<Progress> for Element {
    fn from(progress: Progress) -> Self {
        progress.into_element()
    }
}
