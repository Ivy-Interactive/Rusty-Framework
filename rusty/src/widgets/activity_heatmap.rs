use crate::core::event_registry::EventRegistry;
use crate::shared::{Color, NamedColor};
use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// Whether each cell covers a day or an hour.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActivityInterval {
    #[default]
    Daily,
    Hourly,
}

/// One data point of an [`ActivityHeatmap`].
///
/// `date` is an ISO-8601 date string (`YYYY-MM-DD`): Rusty has no date type and
/// no `chrono` dependency, so Ivy's `DateOnly` is carried as text. `hour` is only
/// meaningful when the heatmap's interval is [`ActivityInterval::Hourly`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Activity {
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hour: Option<u8>,
    pub count: u32,
}

impl Activity {
    pub fn new(date: &str, count: u32) -> Self {
        Activity {
            date: date.to_string(),
            hour: None,
            count,
        }
    }

    pub fn hour(mut self, hour: u8) -> Self {
        self.hour = Some(hour);
        self
    }
}

/// A GitHub-style contribution grid over a series of [`Activity`] counts.
///
/// Days absent from `data` render as zero, so only active days need supplying.
#[derive(Clone, Serialize, Deserialize)]
pub struct ActivityHeatmap {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub data: Vec<Activity>,
    pub color_scheme: Color,
    pub show_tooltip: bool,
    pub show_month_labels: bool,
    pub show_day_labels: bool,
    pub localize: bool,
    pub interval: ActivityInterval,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    #[serde(skip)]
    pub on_day_click: Option<Arc<dyn Fn(Activity) + Send + Sync>>,
}

impl std::fmt::Debug for ActivityHeatmap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActivityHeatmap")
            .field("data", &self.data.len())
            .field("interval", &self.interval)
            .finish()
    }
}

impl ActivityHeatmap {
    pub fn new() -> Self {
        ActivityHeatmap {
            id: None,
            data: Vec::new(),
            color_scheme: Color::Named(NamedColor::Primary),
            show_tooltip: true,
            show_month_labels: true,
            show_day_labels: true,
            localize: false,
            interval: ActivityInterval::Daily,
            value_label: None,
            start_date: None,
            end_date: None,
            on_day_click: None,
        }
    }

    pub fn data(mut self, data: Vec<Activity>) -> Self {
        self.data = data;
        self
    }

    pub fn color_scheme(mut self, scheme: Color) -> Self {
        self.color_scheme = scheme;
        self
    }

    pub fn show_tooltip(mut self, show: bool) -> Self {
        self.show_tooltip = show;
        self
    }

    pub fn show_month_labels(mut self, show: bool) -> Self {
        self.show_month_labels = show;
        self
    }

    pub fn show_day_labels(mut self, show: bool) -> Self {
        self.show_day_labels = show;
        self
    }

    pub fn localize(mut self, localize: bool) -> Self {
        self.localize = localize;
        self
    }

    pub fn interval(mut self, interval: ActivityInterval) -> Self {
        self.interval = interval;
        self
    }

    pub fn value_label(mut self, label: &str) -> Self {
        self.value_label = Some(label.to_string());
        self
    }

    /// The first date rendered, as ISO-8601 `YYYY-MM-DD`.
    pub fn start_date(mut self, date: &str) -> Self {
        self.start_date = Some(date.to_string());
        self
    }

    /// The last date rendered, as ISO-8601 `YYYY-MM-DD`.
    pub fn end_date(mut self, date: &str) -> Self {
        self.end_date = Some(date.to_string());
        self
    }

    pub fn on_day_click(mut self, handler: impl Fn(Activity) + Send + Sync + 'static) -> Self {
        self.on_day_click = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for ActivityHeatmap {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetData for ActivityHeatmap {
    fn widget_type(&self) -> &str {
        "activity_heatmap"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "activity_heatmap",
            "id": self.id,
            "data": self.data,
            "colorScheme": self.color_scheme,
            "showTooltip": self.show_tooltip,
            "showMonthLabels": self.show_month_labels,
            "showDayLabels": self.show_day_labels,
            "localize": self.localize,
            "interval": self.interval,
            "valueLabel": self.value_label,
            "startDate": self.start_date,
            "endDate": self.end_date,
            "hasOnDayClick": self.on_day_click.is_some(),
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

    fn register_events(&self, widget_id: &str, registry: &mut EventRegistry) {
        if let Some(handler) = &self.on_day_click {
            let handler = handler.clone();
            registry.register(
                widget_id,
                "dayclick",
                Arc::new(move |args| {
                    if let Ok(parsed) = serde_json::from_value::<Activity>(args) {
                        handler(parsed);
                    }
                }),
            );
        }
    }
}

impl From<ActivityHeatmap> for Element {
    fn from(heatmap: ActivityHeatmap) -> Self {
        heatmap.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::BuildContext;
    use std::sync::Mutex;

    fn sample_data() -> Vec<Activity> {
        vec![
            Activity::new("2026-01-01", 3),
            Activity::new("2026-01-02", 7),
        ]
    }

    #[test]
    fn test_activity_heatmap_builder_round_trip() {
        let heatmap = ActivityHeatmap::new()
            .data(sample_data())
            .color_scheme(Color::hex("#00ff00"))
            .show_tooltip(false)
            .show_month_labels(false)
            .show_day_labels(false)
            .localize(true)
            .interval(ActivityInterval::Hourly)
            .value_label("commits")
            .start_date("2026-01-01")
            .end_date("2026-12-31");

        assert_eq!(heatmap.data.len(), 2);
        assert_eq!(heatmap.color_scheme, Color::hex("#00ff00"));
        assert!(!heatmap.show_tooltip);
        assert!(!heatmap.show_month_labels);
        assert!(!heatmap.show_day_labels);
        assert!(heatmap.localize);
        assert_eq!(heatmap.interval, ActivityInterval::Hourly);
        assert_eq!(heatmap.value_label.as_deref(), Some("commits"));
        assert_eq!(heatmap.start_date.as_deref(), Some("2026-01-01"));
        assert_eq!(heatmap.end_date.as_deref(), Some("2026-12-31"));
    }

    #[test]
    fn test_activity_heatmap_defaults() {
        let heatmap = ActivityHeatmap::default();
        assert!(heatmap.data.is_empty());
        assert_eq!(heatmap.color_scheme, Color::Named(NamedColor::Primary));
        assert!(heatmap.show_tooltip);
        assert!(heatmap.show_month_labels);
        assert!(heatmap.show_day_labels);
        assert!(!heatmap.localize);
        assert_eq!(heatmap.interval, ActivityInterval::Daily);
    }

    #[test]
    fn test_activity_hour_builder() {
        let activity = Activity::new("2026-01-01", 2).hour(14);
        assert_eq!(activity.hour, Some(14));

        let json = serde_json::to_value(&activity).unwrap();
        assert_eq!(json["date"], "2026-01-01");
        assert_eq!(json["hour"], 14);
        assert_eq!(json["count"], 2);
    }

    #[test]
    fn test_activity_omits_hour_when_absent() {
        let json = serde_json::to_value(Activity::new("2026-01-01", 1)).unwrap();
        assert!(json.get("hour").is_none());
    }

    #[test]
    fn test_activity_heatmap_to_json_keys() {
        let json = ActivityHeatmap::new()
            .data(sample_data())
            .interval(ActivityInterval::Hourly)
            .value_label("builds")
            .on_day_click(|_| {})
            .to_json();

        assert_eq!(json["type"], "activity_heatmap");
        assert_eq!(json["interval"], "hourly");
        assert_eq!(json["valueLabel"], "builds");
        assert_eq!(json["showTooltip"], true);
        assert_eq!(json["data"][1]["count"], 7);
        assert_eq!(json["colorScheme"], "primary");
        assert_eq!(json["hasOnDayClick"], true);
    }

    #[test]
    fn test_activity_heatmap_json_without_handler() {
        assert_eq!(ActivityHeatmap::new().to_json()["hasOnDayClick"], false);
    }

    #[test]
    fn test_activity_heatmap_assign_ids() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let mut element: Element = ActivityHeatmap::new().into();
        element.assign_ids(&mut ctx);
        if let Element::Widget(ref w) = element {
            assert_eq!(w.get_id(), Some("w-0"));
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_activity_heatmap_day_click_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received = Arc::new(Mutex::new(None::<Activity>));
        let received_clone = received.clone();
        let mut element: Element = ActivityHeatmap::new()
            .data(sample_data())
            .on_day_click(move |activity| {
                *received_clone.lock().unwrap() = Some(activity);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch(
            "w-0",
            "dayclick",
            json!({"date": "2026-01-02", "hour": 9, "count": 7})
        ));

        let activity = received
            .lock()
            .unwrap()
            .clone()
            .expect("handler not called");
        assert_eq!(activity.date, "2026-01-02");
        assert_eq!(activity.hour, Some(9));
        assert_eq!(activity.count, 7);
    }
}
