use crate::shared::{Color, Size};
use crate::views::view::{BuildContext, Element, WidgetData};
use rusty_macros::Widget;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

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

/// One segment of a [`StackedProgress`] bar.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressSegment {
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl ProgressSegment {
    pub fn new(value: f64) -> Self {
        ProgressSegment {
            value,
            color: None,
            label: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }
}

/// A multi-segment progress bar where each segment is its own value/color/label.
#[derive(Clone, Serialize, Deserialize, Widget)]
pub struct StackedProgress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    pub segments: Vec<ProgressSegment>,
    #[prop]
    pub bar_height: f64,
    #[prop]
    pub show_labels: bool,
    #[prop]
    pub rounded: bool,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<usize>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Size>,
    #[event(arg = "value")]
    #[serde(skip)]
    pub on_select: Option<Arc<dyn Fn(usize) + Send + Sync>>,
}

impl std::fmt::Debug for StackedProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StackedProgress")
            .field("segments", &self.segments.len())
            .field("selected", &self.selected)
            .finish()
    }
}

impl StackedProgress {
    pub fn new() -> Self {
        StackedProgress {
            id: None,
            segments: Vec::new(),
            bar_height: 8.0,
            show_labels: false,
            rounded: true,
            selected: None,
            width: None,
            on_select: None,
        }
    }

    pub fn segments(mut self, segments: Vec<ProgressSegment>) -> Self {
        self.segments = segments;
        self
    }

    pub fn segment(mut self, segment: ProgressSegment) -> Self {
        self.segments.push(segment);
        self
    }

    pub fn bar_height(mut self, bar_height: f64) -> Self {
        self.bar_height = bar_height;
        self
    }

    pub fn show_labels(mut self, show_labels: bool) -> Self {
        self.show_labels = show_labels;
        self
    }

    pub fn rounded(mut self, rounded: bool) -> Self {
        self.rounded = rounded;
        self
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = Some(selected);
        self
    }

    pub fn width(mut self, width: Size) -> Self {
        self.width = Some(width);
        self
    }

    pub fn on_select(mut self, handler: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_select = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for StackedProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl From<StackedProgress> for Element {
    fn from(stacked_progress: StackedProgress) -> Self {
        stacked_progress.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::shared::NamedColor;

    fn sample_segments() -> Vec<ProgressSegment> {
        vec![
            ProgressSegment::new(3.0)
                .label("Done")
                .color(Color::Named(NamedColor::Success)),
            ProgressSegment::new(2.0).label("In progress"),
            ProgressSegment::new(5.0).label("Todo"),
        ]
    }

    #[test]
    fn test_stacked_progress_builder() {
        let bar = StackedProgress::new()
            .segments(sample_segments())
            .bar_height(12.0)
            .show_labels(true)
            .rounded(false)
            .selected(1)
            .width(Size::Percent(80.0));

        assert_eq!(bar.segments.len(), 3);
        assert_eq!(bar.bar_height, 12.0);
        assert!(bar.show_labels);
        assert!(!bar.rounded);
        assert_eq!(bar.selected, Some(1));
        assert_eq!(bar.width, Some(Size::Percent(80.0)));
    }

    #[test]
    fn test_stacked_progress_json() {
        let json = StackedProgress::new()
            .segments(sample_segments())
            .bar_height(12.0)
            .width(Size::Px(200.0))
            .to_json();

        assert_eq!(json["type"], "stacked_progress");
        assert_eq!(json["segments"][0]["label"], "Done");
        assert_eq!(json["barHeight"], 12.0);
        assert_eq!(json["width"], "200px");
        assert_eq!(json["hasOnSelect"], false);
    }

    #[test]
    fn test_stacked_progress_has_on_select_only_when_set() {
        assert_eq!(StackedProgress::new().to_json()["hasOnSelect"], false);
        assert_eq!(
            StackedProgress::new().on_select(|_| {}).to_json()["hasOnSelect"],
            true
        );
    }

    #[test]
    fn test_stacked_progress_select_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received = Arc::new(std::sync::Mutex::new(None::<usize>));
        let received_clone = received.clone();
        let mut element: Element = StackedProgress::new()
            .segments(sample_segments())
            .on_select(move |index| {
                *received_clone.lock().unwrap() = Some(index);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "select", json!({"value": 1})));
        assert_eq!(*received.lock().unwrap(), Some(1));
    }

    #[test]
    fn test_stacked_progress_select_dispatch_ignores_malformed_payload() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let hits = Arc::new(std::sync::Mutex::new(0usize));
        let hits_clone = hits.clone();
        let mut element: Element = StackedProgress::new()
            .on_select(move |_| {
                *hits_clone.lock().unwrap() += 1;
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        let handled = registry.dispatch("w-0", "select", json!({"value": "not-a-number"}));
        assert!(handled);
        assert_eq!(*hits.lock().unwrap(), 0);
    }
}
