//! Widget type name mapping between Rusty and Ivy Framework.
//!
//! Rusty emits widget type names in `snake_case` (see `WidgetData::to_json`).
//! The vendored Ivy React frontend at `src/frontend` is keyed `"Ivy.PascalCase"`
//! (e.g., `"Ivy.DataTable"`, `"Ivy.Terminal"`).
//!
//! This module records the mapping so it exists in one place when someone wires
//! `src/frontend` to a Rusty server. Nothing in Rusty consumes this today.
//!
//! **Note:** Some widgets added by later plans (e.g., Plan 00037) may not have
//! entries here until they are backfilled. The exhaustiveness test in this module
//! will flag them.

use serde_json::Value;

/// How a Rusty widget type name maps onto Ivy Framework's `widgetMap.ts` keys.
///
/// Rusty's wire format is snake_case (see `WidgetData::to_json`); Ivy's React
/// registry is keyed "Ivy.PascalCase". Nothing in Rusty consumes this today --
/// it exists so the mapping is decided once, in one place, for whoever wires
/// `src/frontend` to a Rusty server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IvyWidget {
    /// A single Ivy key.
    One(&'static str),
    /// Resolved from a prop; see `ivy_widget_for`.
    ByProp { prop: &'static str },
    /// No Ivy counterpart exists.
    RustOnly,
}

/// Maps a Rusty `type` value to its Ivy key(s). Returns `None` for unknown names.
///
/// # Examples
///
/// ```
/// use rusty::shared::{ivy_widget, IvyWidget};
///
/// // Mechanical mappings
/// assert_eq!(ivy_widget("badge"), Some(IvyWidget::One("Ivy.Badge")));
/// assert_eq!(ivy_widget("button"), Some(IvyWidget::One("Ivy.Button")));
///
/// // Renamed widgets
/// assert_eq!(ivy_widget("select"), Some(IvyWidget::One("Ivy.SelectInput")));
/// assert_eq!(ivy_widget("checkbox"), Some(IvyWidget::One("Ivy.BoolInput")));
///
/// // Layout requires resolving by direction prop
/// match ivy_widget("layout") {
///     Some(IvyWidget::ByProp { prop }) if prop == "direction" => {},
///     _ => panic!("layout should be ByProp"),
/// }
///
/// // Rust-only widgets
/// assert_eq!(ivy_widget("qr_code"), Some(IvyWidget::RustOnly));
/// ```
pub fn ivy_widget(rusty_type: &str) -> Option<IvyWidget> {
    match rusty_type {
        // Mechanical snake_case → Ivy.PascalCase mappings (14 widgets)
        "badge" => Some(IvyWidget::One("Ivy.Badge")),
        "button" => Some(IvyWidget::One("Ivy.Button")),
        "card" => Some(IvyWidget::One("Ivy.Card")),
        "data_table" => Some(IvyWidget::One("Ivy.DataTable")),
        "dialog" => Some(IvyWidget::One("Ivy.Dialog")),
        "field" => Some(IvyWidget::One("Ivy.Field")),
        "form" => Some(IvyWidget::One("Ivy.Form")),
        "number_input" => Some(IvyWidget::One("Ivy.NumberInput")),
        "progress" => Some(IvyWidget::One("Ivy.Progress")),
        "table" => Some(IvyWidget::One("Ivy.Table")),
        "terminal" => Some(IvyWidget::One("Ivy.Terminal")),
        "text_block" => Some(IvyWidget::One("Ivy.TextBlock")),
        "text_input" => Some(IvyWidget::One("Ivy.TextInput")),
        "tooltip" => Some(IvyWidget::One("Ivy.Tooltip")),

        // Renamed widgets (2)
        "select" => Some(IvyWidget::One("Ivy.SelectInput")),
        "checkbox" => Some(IvyWidget::One("Ivy.BoolInput")),

        // One-to-many mapping (1)
        "layout" => Some(IvyWidget::ByProp { prop: "direction" }),

        // Rust-only widgets with no Ivy counterpart (4)
        // - activity_heatmap: no heatmap widget in widgetMap.ts
        // - diff_view: no diff widget in widgetMap.ts
        // - qr_code: no Ivy.QrCode or Ivy.QRCode in widgetMap.ts
        // - rich_text_input: Ivy.RichTextBlock is read-only; Ivy.ContentInput
        //   and Ivy.CodeInput are different widgets
        "activity_heatmap" => Some(IvyWidget::RustOnly),
        "diff_view" => Some(IvyWidget::RustOnly),
        "qr_code" => Some(IvyWidget::RustOnly),
        "rich_text_input" => Some(IvyWidget::RustOnly),

        _ => None,
    }
}

/// Resolves the concrete Ivy key for a serialized widget, reading the
/// discriminating prop where the mapping is one-to-many.
///
/// # Examples
///
/// ```
/// use rusty::widgets::Layout;
/// use rusty::views::view::WidgetData;
/// use rusty::shared::ivy_widget_for;
///
/// let vertical = Layout::vertical().to_json();
/// assert_eq!(ivy_widget_for(&vertical), Some("Ivy.StackLayout"));
///
/// let horizontal = Layout::horizontal().to_json();
/// assert_eq!(ivy_widget_for(&horizontal), Some("Ivy.StackLayout"));
///
/// let grid = Layout::grid(2);
/// assert_eq!(ivy_widget_for(&grid.to_json()), Some("Ivy.GridLayout"));
/// ```
pub fn ivy_widget_for(widget_json: &Value) -> Option<&'static str> {
    let type_str = widget_json.get("type")?.as_str()?;

    match ivy_widget(type_str)? {
        IvyWidget::One(key) => Some(key),
        IvyWidget::ByProp { prop } => {
            // Currently only "layout" is ByProp, which discriminates on "direction"
            if type_str == "layout" && prop == "direction" {
                let direction = widget_json.get("direction")?.as_str()?;
                // Match case-insensitively because LayoutDirection serializes with
                // #[serde(rename_all = "camelCase")], yielding "vertical" / "horizontal" / "grid"
                match direction.to_lowercase().as_str() {
                    "grid" => Some("Ivy.GridLayout"),
                    _ => Some("Ivy.StackLayout"), // "vertical" or "horizontal"
                }
            } else {
                None
            }
        }
        IvyWidget::RustOnly => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::view::WidgetData;
    use crate::widgets::*;

    #[test]
    fn every_widget_type_is_mapped() {
        // Build one instance of each of the 21 widgets and verify ivy_widget returns Some
        let widgets: Vec<Box<dyn WidgetData>> = vec![
            Box::new(ActivityHeatmap::new()),
            Box::new(Badge::new("test")),
            Box::new(Button::new("test")),
            Box::new(Card::new()),
            Box::new(DataTable::new(vec![])),
            Box::new(Dialog::new(true)),
            Box::new(DiffView::new()),
            Box::new(Form::new()),
            Box::new(Field::new("test", TextBlock::new("x"))),
            Box::new(TextInput::new()),
            Box::new(NumberInput::new()),
            Box::new(Select::new(vec![])),
            Box::new(Checkbox::new(false)),
            Box::new(Layout::vertical()),
            Box::new(Progress::new(0.5)),
            Box::new(QrCode::new("test")),
            Box::new(RichTextInput::new()),
            Box::new(Table::new(vec![])),
            Box::new(Terminal::new()),
            Box::new(TextBlock::new("test")),
            Box::new(Tooltip::new("test", TextBlock::new("x"))),
        ];

        for widget in widgets {
            let json = widget.to_json();
            let type_str = json["type"].as_str().expect("type should be a string");
            assert!(
                ivy_widget(type_str).is_some(),
                "Widget type '{}' is not mapped in ivy_widget",
                type_str
            );
        }
    }

    #[test]
    fn layout_resolves_by_direction() {
        let vertical = Layout::vertical().to_json();
        assert_eq!(ivy_widget_for(&vertical), Some("Ivy.StackLayout"));

        let horizontal = Layout::horizontal().to_json();
        assert_eq!(ivy_widget_for(&horizontal), Some("Ivy.StackLayout"));

        let mut grid = Layout::vertical();
        grid.direction = crate::widgets::layout::LayoutDirection::Grid;
        assert_eq!(ivy_widget_for(&grid.to_json()), Some("Ivy.GridLayout"));
    }

    #[test]
    fn layout_direction_serializes_as_expected() {
        // Pin the serialized direction spelling so a future rename_all doesn't silently break it
        let vertical = Layout::vertical().to_json();
        assert_eq!(vertical["direction"], "vertical");

        let horizontal = Layout::horizontal().to_json();
        assert_eq!(horizontal["direction"], "horizontal");

        let mut grid = Layout::vertical();
        grid.direction = crate::widgets::layout::LayoutDirection::Grid;
        assert_eq!(grid.to_json()["direction"], "grid");
    }

    #[test]
    fn rust_only_widgets_have_no_ivy_key() {
        let activity_heatmap = ActivityHeatmap::new().to_json();
        assert_eq!(ivy_widget_for(&activity_heatmap), None);

        let diff_view = DiffView::new().to_json();
        assert_eq!(ivy_widget_for(&diff_view), None);

        let qr_code = QrCode::new("test").to_json();
        assert_eq!(ivy_widget_for(&qr_code), None);

        let rich_text_input = RichTextInput::new().to_json();
        assert_eq!(ivy_widget_for(&rich_text_input), None);
    }

    #[test]
    fn unknown_type_is_none() {
        assert_eq!(ivy_widget("not_a_widget"), None);
        assert_eq!(ivy_widget(""), None);
    }

    #[test]
    fn renamed_widgets_map_to_ivy_names() {
        assert_eq!(
            ivy_widget("select"),
            Some(IvyWidget::One("Ivy.SelectInput"))
        );
        assert_eq!(
            ivy_widget("checkbox"),
            Some(IvyWidget::One("Ivy.BoolInput"))
        );
    }
}
