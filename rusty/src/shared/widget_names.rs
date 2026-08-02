//! Widget type name mapping between Rusty and Ivy Framework.
//!
//! Rusty emits widget type names in `snake_case` (see `WidgetData::to_json`).
//! The vendored Ivy React frontend at `src/frontend` is keyed `"Ivy.PascalCase"`
//! (e.g., `"Ivy.DataTable"`, `"Ivy.Terminal"`).
//!
//! This module records the mapping so it exists in one place when someone wires
//! `src/frontend` to a Rusty server. Nothing in Rusty consumes this today, except
//! [`crate::shared::ivy_node`], which builds on it to reshape a whole widget tree.
//!
//! All 38 Rusty widget types have an entry. `every_widget_type_is_mapped` derives its
//! list by scanning `rusty/src/widgets/*.rs` for both ways a widget declares its wire
//! name -- the `"type": "..."` literal of a hand-written `to_json` and the name
//! `#[derive(Widget)]` generates -- so a widget added without an entry here fails the
//! test rather than being silently unmapped.

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
    /// A single Ivy key plus a prop the adapter must inject, for Rusty types that
    /// Ivy models as a variant of a more general widget (`text_area` -> `Ivy.TextInput`
    /// with `variant: "Textarea"`). `ByProp` cannot express this: it *reads* a prop
    /// Rust already sent, whereas this *synthesises* one Rust never sends.
    WithProp {
        key: &'static str,
        prop: &'static str,
        value: &'static str,
    },
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
/// assert_eq!(ivy_widget("container"), Some(IvyWidget::One("Ivy.Box")));
///
/// // Layout requires resolving by direction prop
/// match ivy_widget("layout") {
///     Some(IvyWidget::ByProp { prop }) if prop == "direction" => {},
///     _ => panic!("layout should be ByProp"),
/// }
///
/// // text_area is a variant of Ivy.TextInput, so it carries a prop to inject
/// assert_eq!(
///     ivy_widget("text_area"),
///     Some(IvyWidget::WithProp {
///         key: "Ivy.TextInput",
///         prop: "variant",
///         value: "Textarea",
///     })
/// );
///
/// // Rust-only widgets
/// assert_eq!(ivy_widget("qr_code"), Some(IvyWidget::RustOnly));
/// assert_eq!(ivy_widget("slider"), Some(IvyWidget::RustOnly));
/// ```
pub fn ivy_widget(rusty_type: &str) -> Option<IvyWidget> {
    match rusty_type {
        // Mechanical snake_case → Ivy.PascalCase mappings (25 widgets)
        "avatar" => Some(IvyWidget::One("Ivy.Avatar")),
        "badge" => Some(IvyWidget::One("Ivy.Badge")),
        "button" => Some(IvyWidget::One("Ivy.Button")),
        "callout" => Some(IvyWidget::One("Ivy.Callout")),
        "card" => Some(IvyWidget::One("Ivy.Card")),
        "color_input" => Some(IvyWidget::One("Ivy.ColorInput")),
        "data_table" => Some(IvyWidget::One("Ivy.DataTable")),
        "dialog" => Some(IvyWidget::One("Ivy.Dialog")),
        "expandable" => Some(IvyWidget::One("Ivy.Expandable")),
        "field" => Some(IvyWidget::One("Ivy.Field")),
        "form" => Some(IvyWidget::One("Ivy.Form")),
        "icon" => Some(IvyWidget::One("Ivy.Icon")),
        "image" => Some(IvyWidget::One("Ivy.Image")),
        "list" => Some(IvyWidget::One("Ivy.List")),
        "list_item" => Some(IvyWidget::One("Ivy.ListItem")),
        "number_input" => Some(IvyWidget::One("Ivy.NumberInput")),
        "progress" => Some(IvyWidget::One("Ivy.Progress")),
        "separator" => Some(IvyWidget::One("Ivy.Separator")),
        "skeleton" => Some(IvyWidget::One("Ivy.Skeleton")),
        "spacer" => Some(IvyWidget::One("Ivy.Spacer")),
        "table" => Some(IvyWidget::One("Ivy.Table")),
        "terminal" => Some(IvyWidget::One("Ivy.Terminal")),
        "text_block" => Some(IvyWidget::One("Ivy.TextBlock")),
        "text_input" => Some(IvyWidget::One("Ivy.TextInput")),
        "tooltip" => Some(IvyWidget::One("Ivy.Tooltip")),

        // Renamed widgets (4)
        "select" => Some(IvyWidget::One("Ivy.SelectInput")),
        "checkbox" => Some(IvyWidget::One("Ivy.BoolInput")),
        // No Ivy.Container exists; Ivy.Box is the general-purpose box.
        "container" => Some(IvyWidget::One("Ivy.Box")),
        // Ivy.DateRangeInput is a *different* widget (two-value range), not this one.
        "date_input" => Some(IvyWidget::One("Ivy.DateTimeInput")),

        // One-to-many mapping (1)
        "layout" => Some(IvyWidget::ByProp { prop: "direction" }),

        // Collapses into a variant of another Ivy widget (1)
        // Ivy has no Ivy.TextArea; it is TextInputVariant.Textarea of Ivy.TextInput.
        "text_area" => Some(IvyWidget::WithProp {
            key: "Ivy.TextInput",
            prop: "variant",
            value: "Textarea",
        }),

        // Rust-only widgets with no Ivy counterpart (7)
        // - activity_heatmap: no heatmap widget in widgetMap.ts
        // - diff_view: no diff widget in widgetMap.ts
        // - multi_select: no Ivy.MultiSelect; Ivy.SelectInput is single-value
        // - qr_code: no Ivy.QrCode or Ivy.QRCode in widgetMap.ts
        // - radio_group: no Ivy.RadioGroup in widgetMap.ts
        // - rich_text_input: Ivy.RichTextBlock is read-only; Ivy.ContentInput
        //   and Ivy.CodeInput are different widgets
        // - slider: no Ivy.Slider in widgetMap.ts
        "activity_heatmap" => Some(IvyWidget::RustOnly),
        "diff_view" => Some(IvyWidget::RustOnly),
        "multi_select" => Some(IvyWidget::RustOnly),
        "qr_code" => Some(IvyWidget::RustOnly),
        "radio_group" => Some(IvyWidget::RustOnly),
        "rich_text_input" => Some(IvyWidget::RustOnly),
        "slider" => Some(IvyWidget::RustOnly),

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
        // The injected prop is the adapter's job (see `shared::ivy_node`); the key
        // is all a caller resolving a type name needs.
        IvyWidget::WithProp { key, .. } => Some(key),
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
    use std::fs;
    use std::path::Path;

    /// Collects every widget type name from the source of truth: `rusty/src/widgets/*.rs`.
    ///
    /// The previous version of `every_widget_type_is_mapped` restated the inventory as a
    /// hardcoded 21-element `Vec` of constructors. When Plan 00037 took the widget count
    /// to 38, the test stayed green while 17 types went unmapped -- an "exhaustiveness"
    /// assertion that could not see the thing it was meant to guard. Deriving the list
    /// means a new widget cannot be added without either mapping it or failing here.
    ///
    /// A widget declares its wire name in one of two ways, and both must be scanned:
    /// a hand-written `to_json` spells the literal out, while `#[derive(Widget)]`
    /// generates it and so leaves no literal behind.
    fn widget_types_from_sources() -> Vec<String> {
        let widgets_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/widgets");
        let mut types = Vec::new();

        for entry in fs::read_dir(&widgets_dir).expect("rusty/src/widgets should be readable") {
            let path = entry.expect("dir entry should be readable").path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let source = fs::read_to_string(&path).expect("widget source should be readable");
            for name in literal_types(&source)
                .into_iter()
                .chain(derived_types(&source))
            {
                if !types.contains(&name) {
                    types.push(name);
                }
            }
        }

        types.sort();
        types
    }

    /// The `"type": "snake_case"` literals a hand-written `to_json` spells out.
    fn literal_types(source: &str) -> Vec<String> {
        let mut types = Vec::new();
        for line in source.lines() {
            let Some(rest) = line.split_once("\"type\": \"") else {
                continue;
            };
            let Some((name, _)) = rest.1.split_once('"') else {
                continue;
            };
            // to_json bodies use only lowercase + underscore; anything else is
            // interpolation or a test fixture, not a real wire type name.
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                types.push(name.to_string());
            }
        }
        types
    }

    /// The names `#[derive(Widget)]` generates, which appear nowhere in the source
    /// it is applied to.
    ///
    /// Plan 00093 moved 12 widgets onto the derive and the literal scan went blind to
    /// every one of them -- `button` included -- so the count fell to 26 and both
    /// assertions below started failing. The derive takes the name from
    /// `#[widget(type = "...")]` when present and from the struct name otherwise.
    fn derived_types(source: &str) -> Vec<String> {
        let mut types = Vec::new();
        let mut lines = source.lines();

        while let Some(line) = lines.next() {
            if !derives_widget(line) {
                continue;
            }
            // Walk the attributes between the derive and the struct it sits on.
            let mut explicit: Option<String> = None;
            for next in lines.by_ref() {
                let next = next.trim();
                if let Some(rest) = next.strip_prefix("#[widget(type = \"") {
                    explicit = rest.split_once('"').map(|(name, _)| name.to_string());
                    continue;
                }
                if next.starts_with("#[") || next.starts_with("///") {
                    continue;
                }
                if let Some(name) = struct_name(next) {
                    types.push(explicit.take().unwrap_or_else(|| to_snake_case(&name)));
                }
                break;
            }
        }

        types
    }

    /// Whether a line is a `#[derive(..)]` naming `Widget` itself -- not `WidgetData`,
    /// and not some type whose name merely ends in `Widget`.
    fn derives_widget(line: &str) -> bool {
        let Some(rest) = line.trim().strip_prefix("#[derive(") else {
            return false;
        };
        let Some((list, _)) = rest.split_once(")]") else {
            return false;
        };
        list.split(',').any(|item| item.trim() == "Widget")
    }

    /// `pub struct Button {` -> `Button`.
    fn struct_name(line: &str) -> Option<String> {
        let rest = line
            .strip_prefix("pub struct ")
            .or_else(|| line.strip_prefix("struct "))?;
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        (!name.is_empty()).then_some(name)
    }

    /// The derive's own struct-name-to-wire-name rule, mirrored from
    /// `rusty_macros`' `to_snake_case` so the two cannot disagree silently.
    fn to_snake_case(name: &str) -> String {
        let mut out = String::new();
        for (i, ch) in name.chars().enumerate() {
            if ch.is_uppercase() && i > 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        }
        out
    }

    #[test]
    fn every_widget_type_is_mapped() {
        let types = widget_types_from_sources();

        // Guard against the scan itself breaking (a rename of the to_json literal
        // style, a moved directory): an empty or tiny list would pass vacuously,
        // which is the exact failure this test was rewritten to eliminate.
        assert!(
            types.len() >= 38,
            "expected at least 38 widget types scanned from rusty/src/widgets, found {}: {:?}. \
             If the way a widget declares its wire name changed -- a renamed to_json \
             literal, a new alternative to #[derive(Widget)] -- fix this scan.",
            types.len(),
            types
        );

        let unmapped: Vec<&String> = types
            .iter()
            .filter(|t| ivy_widget(t).is_none())
            .collect::<Vec<_>>();

        assert!(
            unmapped.is_empty(),
            "{} widget type(s) have no ivy_widget entry: {:?}. \
             Add them to ivy_widget -- IvyWidget::RustOnly if Ivy has no counterpart.",
            unmapped.len(),
            unmapped
        );
    }

    #[test]
    fn widget_type_scan_finds_known_widgets() {
        // Negative control for the scan above: prove it actually reads the sources
        // rather than returning a list that happens to be long enough. The names are
        // split by declaration style on purpose -- a scan that lost either branch
        // would still find the other and the count alone might stay over 38.
        let types = widget_types_from_sources();
        for expected in [
            // hand-written `"type": "..."` literals
            "text_area",
            "slider",
            "progress",
            // generated by #[derive(Widget)] from the struct name
            "button",
            "list_item",
            "expandable",
            // generated by #[derive(Widget)] from a #[widget(type = "...")] override
            "icon",
        ] {
            assert!(
                types.contains(&expected.to_string()),
                "scan missed '{}'; found {:?}",
                expected,
                types
            );
        }
        assert!(!types.contains(&"not_a_widget".to_string()));
    }

    #[test]
    fn derived_type_scan_reads_both_the_struct_name_and_the_override() {
        // Unit-test the derive branch against fixtures rather than the live tree, so
        // a future widget migration cannot quietly make this control vacuous.
        let source = r#"
#[derive(Debug, Clone, Serialize, Widget)]
pub struct TextBlock {
    id: Option<String>,
}

/// A doc comment and an unrelated attribute between derive and struct.
#[derive(Clone, Widget)]
#[widget(type = "icon")]
#[serde(rename_all = "camelCase")]
pub struct IconWidget {}

#[derive(Clone, Serialize)]
pub struct NotAWidget {}

#[derive(Clone, WidgetData)]
pub struct AlsoNotAWidget {}
"#;

        assert_eq!(derived_types(source), vec!["text_block", "icon"]);
        assert!(literal_types(source).is_empty());
    }

    #[test]
    fn constructed_widgets_all_resolve() {
        // Complements the source scan by going through the real builders, so a type
        // string that exists in source but is unreachable through the public API
        // still cannot drift.
        let widgets: Vec<Box<dyn WidgetData>> = vec![
            Box::new(ActivityHeatmap::new()),
            Box::new(Avatar::new("AB")),
            Box::new(Badge::new("test")),
            Box::new(Button::new("test")),
            Box::new(Callout::new()),
            Box::new(Card::new()),
            Box::new(Checkbox::new(false)),
            Box::new(ColorInput::new()),
            Box::new(Container::new()),
            Box::new(DataTable::new(vec![])),
            Box::new(DateInput::new()),
            Box::new(Dialog::new(true)),
            Box::new(DiffView::new()),
            Box::new(Expandable::new("test")),
            Box::new(Field::new("test", TextBlock::new("x"))),
            Box::new(Form::new()),
            Box::new(IconWidget::new("check")),
            Box::new(Image::new("a.png")),
            Box::new(Layout::vertical()),
            Box::new(List::new()),
            Box::new(ListItem::new("test")),
            Box::new(MultiSelect::new(vec![])),
            Box::new(NumberInput::new()),
            Box::new(Progress::new(0.5)),
            Box::new(QrCode::new("test")),
            Box::new(RadioGroup::new(vec![])),
            Box::new(RichTextInput::new()),
            Box::new(Select::new(vec![])),
            Box::new(Separator::horizontal()),
            Box::new(Skeleton::new()),
            Box::new(Slider::new(0.0)),
            Box::new(Spacer::new()),
            Box::new(Table::new(vec![])),
            Box::new(Terminal::new()),
            Box::new(TextArea::new()),
            Box::new(TextBlock::new("test")),
            Box::new(TextInput::new()),
            Box::new(Tooltip::new("test", TextBlock::new("x"))),
        ];

        assert_eq!(
            widgets.len(),
            38,
            "constructed-widget list drifted from the 38 widget types"
        );

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
    fn container_and_date_input_are_renamed() {
        // Neither name is mechanical: Ivy has no "Ivy.Container", and "Ivy.DateRangeInput"
        // is a different widget from the one date_input maps to.
        assert_eq!(ivy_widget("container"), Some(IvyWidget::One("Ivy.Box")));
        assert_eq!(
            ivy_widget("date_input"),
            Some(IvyWidget::One("Ivy.DateTimeInput"))
        );

        assert_eq!(ivy_widget_for(&Container::new().to_json()), Some("Ivy.Box"));
        assert_eq!(
            ivy_widget_for(&DateInput::new().to_json()),
            Some("Ivy.DateTimeInput")
        );
    }

    #[test]
    fn text_area_carries_the_variant_to_inject() {
        assert_eq!(
            ivy_widget("text_area"),
            Some(IvyWidget::WithProp {
                key: "Ivy.TextInput",
                prop: "variant",
                value: "Textarea",
            })
        );
        // ivy_widget_for reports only the key; injecting the prop is ivy_node's job.
        assert_eq!(
            ivy_widget_for(&TextArea::new().to_json()),
            Some("Ivy.TextInput")
        );
    }

    #[test]
    fn newly_backfilled_types_are_not_rust_only() {
        for (rusty_type, expected) in [
            ("avatar", "Ivy.Avatar"),
            ("callout", "Ivy.Callout"),
            ("color_input", "Ivy.ColorInput"),
            ("expandable", "Ivy.Expandable"),
            ("icon", "Ivy.Icon"),
            ("image", "Ivy.Image"),
            ("list", "Ivy.List"),
            ("list_item", "Ivy.ListItem"),
            ("separator", "Ivy.Separator"),
            ("skeleton", "Ivy.Skeleton"),
            ("spacer", "Ivy.Spacer"),
        ] {
            assert_eq!(
                ivy_widget(rusty_type),
                Some(IvyWidget::One(expected)),
                "{} should map to {}",
                rusty_type,
                expected
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

        // Backfilled in this plan: Ivy has no MultiSelect, RadioGroup or Slider key.
        assert_eq!(ivy_widget_for(&MultiSelect::new(vec![]).to_json()), None);
        assert_eq!(ivy_widget_for(&RadioGroup::new(vec![]).to_json()), None);
        assert_eq!(ivy_widget_for(&Slider::new(0.0).to_json()), None);
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
