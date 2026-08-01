//! Translation from a Rusty widget JSON tree to Ivy Framework's `WidgetNode` shape.
//!
//! Rusty and Ivy disagree about node structure in four ways, and this module settles
//! each of them. Nothing in Rusty consumes it today -- like [`crate::shared::widget_names`],
//! it exists so the decisions are made once, in one place, for whoever wires
//! `src/frontend` to a Rusty server. The wire format is unchanged.
//!
//! | | Rusty | Ivy |
//! |---|---|---|
//! | props | flattened to the top level | nested under `props` |
//! | events | `hasOnClick: true` booleans | `events: ["OnClick"]` string array |
//! | enum values | camelCase (`"danger"`) | PascalCase, sometimes renamed (`"Destructive"`) |
//! | single child | `"child": {…}` on `field`/`tooltip` | always `children?: WidgetNode[]` |
//!
//! Everything here is pure: no I/O, no interior mutability, no dependence on a
//! `BuildContext` or an [`crate::core::event_registry::EventRegistry`].
//!
//! # Scope
//!
//! This is the **outbound** direction only (Rusty -> Ivy). The inbound direction
//! (an Ivy event name arriving at Rusty) is already handled by
//! [`crate::core::event_registry::EventName::canonicalize`], which accepts `OnClick`,
//! `onClick` and `click` alike.
//!
//! # Example
//!
//! ```
//! use rusty::shared::to_ivy_node;
//! use rusty::views::view::WidgetData;
//! use rusty::widgets::Button;
//!
//! let button = Button::new("Save").on_click(|| {});
//! let node = to_ivy_node(&button.to_json()).expect("button maps to Ivy.Button");
//!
//! assert_eq!(node["type"], "Ivy.Button");
//! assert_eq!(node["props"]["title"], "Save");
//! assert_eq!(node["events"][0], "OnClick");
//! // `id` and `events` live at the top level; Ivy re-injects them into props itself.
//! assert!(node["props"].get("hasOnClick").is_none());
//! ```

use serde_json::{json, Map, Value};

use super::widget_names::{ivy_widget, IvyWidget};

/// Top-level keys that never become props: they are either structural in Ivy's
/// `WidgetNode` (`type`, `id`, `children`), carry the child subtree (`child`), or are
/// Rusty's own serde plumbing (`kind`).
///
/// `kind` is the internal tag of [`crate::views::view::Element`]
/// (`#[serde(tag = "kind")]`), so every widget reached as a *child* carries
/// `"kind": "widget"` alongside its real props. It describes Rusty's element tree, not
/// the widget, and Ivy has no such field.
const RESERVED_KEYS: [&str; 5] = ["type", "id", "children", "child", "kind"];

/// The Ivy event names that some widget under `src/frontend/src/widgets/` actually
/// reads via `events.includes(...)`. Derived from Rusty's `has<Event>` booleans, but
/// deliberately **not** a blanket `has` strip: Rusty emits four flags Ivy has nowhere
/// to land, and emitting them would suggest a handler Ivy will never invoke.
///
/// Dropped, with the reason each is unreachable:
///
/// - `OnResize`, `OnInput` -- from `terminal`, which *is* mapped (`Ivy.Terminal`), but
///   `TerminalWidget.tsx` reads no `events` prop at all.
/// - `OnToggle` -- from `expandable` (`Ivy.Expandable`); `ExpandableWidget.tsx` takes
///   `id, disabled, open, density, icon, ghost, slots` and no `events`.
/// - `OnDayClick` -- from `activity_heatmap`, which is `IvyWidget::RustOnly`.
/// - `OnLineClick` -- from `diff_view`, which is `IvyWidget::RustOnly`.
///
/// If a future Ivy sync adds one of them, add it here; `unreadable_events_are_dropped`
/// pins the current set so the divergence is noticed rather than assumed.
pub const IVY_EVENT_NAMES: [&str; 8] = [
    "OnBlur",
    "OnCellClick",
    "OnChange",
    "OnClick",
    "OnFocus",
    "OnLinkClick",
    "OnRowAction",
    "OnSubmit",
];

/// Props whose values are Rust enums, and so are safe to recase for Ivy.
///
/// The rule is an allow-list rather than a blanket transform because most props carry
/// user text: title-casing `content` or `title` would corrupt it.
const ENUM_PROPS: [&str; 5] = ["variant", "direction", "density", "color", "orientation"];

/// Maps a Rusty camelCase enum prop value onto Ivy's spelling, for the pairs that
/// differ by more than case.
///
/// Returns `None` when no rename applies, in which case the caller should fall back to
/// title-casing the first character (`"primary"` -> `"Primary"`).
///
/// # Examples
///
/// ```
/// use rusty::shared::ivy_prop_value;
///
/// // Ivy calls the destructive button variant "Destructive", not "Danger".
/// assert_eq!(ivy_prop_value("button", "variant", "danger"), Some("Destructive"));
///
/// // Rust's Density is compact/normal/comfortable; Ivy's is Small/Medium/Large.
/// assert_eq!(ivy_prop_value("button", "density", "normal"), Some("Medium"));
///
/// // No rename needed -- the caller title-cases instead.
/// assert_eq!(ivy_prop_value("button", "variant", "primary"), None);
/// ```
pub fn ivy_prop_value(rusty_type: &str, prop: &str, value: &str) -> Option<&'static str> {
    // `density` is shared::Density on every widget that has it, so it renames uniformly.
    if prop == "density" {
        return match value {
            "compact" => Some("Small"),
            "normal" => Some("Medium"),
            "comfortable" => Some("Large"),
            _ => None,
        };
    }

    match (rusty_type, prop, value) {
        // ButtonVariant::Danger; Ivy's ButtonWidgetProps.variant has no "Danger".
        ("button", "variant", "danger") => Some("Destructive"),
        // BadgeVariant::{Default,Dot}; Ivy's badge variants are
        // Primary/Destructive/Outline/Secondary/Success/Warning/Info. "Outline" is
        // shared, "Default" is spelled "Primary", and "Dot" has no counterpart --
        // left un-renamed so the mismatch stays visible rather than being guessed at.
        ("badge", "variant", "default") => Some("Primary"),
        // CalloutVariant::Error; Ivy's CalloutWidget accepts both "Error" and
        // "Destructive", so the title-cased fallback is already correct -- no entry.

        // TextVariant -> Ivy's TextBlockVariant, which is a different vocabulary:
        // Literal/H1..H6/P/Inline/Block/Blockquote/Monospaced/Lead/Muted/Danger/
        // Warning/Success/Label/Strong/Display.
        ("text_block", "variant", "heading1") => Some("H1"),
        ("text_block", "variant", "heading2") => Some("H2"),
        ("text_block", "variant", "heading3") => Some("H3"),
        ("text_block", "variant", "heading4") => Some("H4"),
        ("text_block", "variant", "paragraph") => Some("P"),
        ("text_block", "variant", "code") => Some("Monospaced"),
        // Ivy renders Markdown through its Lead variant's MarkdownRenderer; there is
        // no "Markdown" key in its variantMap.
        ("text_block", "variant", "markdown") => Some("Lead"),
        // TextVariant::Caption has no Ivy counterpart; "Muted" is the nearest
        // typographic role Ivy ships.
        ("text_block", "variant", "caption") => Some("Muted"),

        _ => None,
    }
}

/// The Ivy event names implied by a widget's `has<Event>` booleans, sorted for
/// determinism.
///
/// Only names in [`IVY_EVENT_NAMES`] are emitted; see that constant for what is
/// dropped and why.
///
/// # Examples
///
/// ```
/// use rusty::shared::ivy_events;
/// use rusty::views::view::WidgetData;
/// use rusty::widgets::{Button, Terminal};
///
/// let clickable = Button::new("Go").on_click(|| {}).to_json();
/// assert_eq!(ivy_events(&clickable), vec!["OnClick"]);
///
/// // A handler-free widget yields an empty list, not a missing one.
/// assert!(ivy_events(&Button::new("Go").to_json()).is_empty());
///
/// // Terminal's OnInput/OnResize have no Ivy landing spot and are dropped.
/// let term = Terminal::new().on_input(|_| {}).on_resize(|_| {}).to_json();
/// assert!(ivy_events(&term).is_empty());
/// ```
pub fn ivy_events(widget_json: &Value) -> Vec<&'static str> {
    let Some(obj) = widget_json.as_object() else {
        return Vec::new();
    };

    let mut events: Vec<&'static str> = obj
        .iter()
        .filter(|(_, v)| v.as_bool() == Some(true))
        .filter_map(|(k, _)| k.strip_prefix("has"))
        .filter_map(|name| IVY_EVENT_NAMES.iter().copied().find(|ivy| *ivy == name))
        .collect();

    events.sort_unstable();
    events.dedup();
    events
}

/// Translates a Rusty widget JSON tree into the shape Ivy's `widgetRenderer` expects:
/// `{ type, id, props: {…}, children, events: [] }`.
///
/// Returns `None` when the widget has no Ivy counterpart ([`IvyWidget::RustOnly`]) or
/// its type is unknown, so a caller can decide whether to skip it or fall back. Child
/// widgets that translate to `None` are dropped from `children` rather than emitted as
/// `null`, because Ivy maps over the array unconditionally.
///
/// # Examples
///
/// ```
/// use rusty::shared::to_ivy_node;
/// use rusty::views::view::WidgetData;
/// use rusty::widgets::{Card, QrCode, TextBlock};
///
/// // Children recurse.
/// let card = Card::new().child(TextBlock::new("hi")).to_json();
/// let node = to_ivy_node(&card).unwrap();
/// assert_eq!(node["children"][0]["type"], "Ivy.TextBlock");
///
/// // Rust-only widgets have no node at all.
/// assert!(to_ivy_node(&QrCode::new("x").to_json()).is_none());
/// ```
pub fn to_ivy_node(widget_json: &Value) -> Option<Value> {
    let obj = widget_json.as_object()?;
    let rusty_type = obj.get("type")?.as_str()?;

    // Resolve the type through widget_names rather than duplicating the mapping.
    let mapping = ivy_widget(rusty_type)?;
    let (ivy_type, injected) = match mapping {
        IvyWidget::One(key) => (key, None),
        IvyWidget::WithProp { key, prop, value } => (key, Some((prop, value))),
        IvyWidget::ByProp { .. } => (super::widget_names::ivy_widget_for(widget_json)?, None),
        IvyWidget::RustOnly => return None,
    };

    let mut props = Map::new();
    for (key, value) in obj {
        // `has*` flags become the events array; reserved keys are structural.
        if RESERVED_KEYS.contains(&key.as_str()) || key.starts_with("has") {
            continue;
        }
        props.insert(key.clone(), translate_prop(rusty_type, key, value));
    }

    // For a WithProp type, synthesise the prop Ivy needs but Rust never sends --
    // without clobbering a value the widget did send.
    if let Some((prop, value)) = injected {
        props
            .entry(prop.to_string())
            .or_insert_with(|| Value::String(value.to_string()));
    }

    Some(json!({
        "type": ivy_type,
        // Ivy's WidgetNode.id is a string; an unbuilt widget has no id yet, and Ivy
        // reads `node.id` for its React key, so send "" rather than null.
        "id": obj.get("id").and_then(Value::as_str).unwrap_or_default(),
        "props": Value::Object(props),
        "children": Value::Array(ivy_children(obj)),
        "events": ivy_events(widget_json)
            .into_iter()
            .map(Value::from)
            .collect::<Vec<_>>(),
    }))
}

/// Recases an enum prop value for Ivy, leaving every other value untouched.
fn translate_prop(rusty_type: &str, prop: &str, value: &Value) -> Value {
    if !ENUM_PROPS.contains(&prop) {
        return value.clone();
    }
    let Some(text) = value.as_str() else {
        // `color` is Color, an untagged enum: a hex string or an { r, g, b, a } object.
        // Only the string form can be an enum name.
        return value.clone();
    };

    if let Some(renamed) = ivy_prop_value(rusty_type, prop, text) {
        return Value::String(renamed.to_string());
    }
    Value::String(title_case_first(text))
}

/// Uppercases the first ASCII character. Applied only to [`ENUM_PROPS`] values, where
/// serde's `rename_all = "camelCase"` guarantees a single lowercase leading word for
/// the single-word variants and leaves multi-word ones to [`ivy_prop_value`].
fn title_case_first(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// Collects a widget's translated children from either `children[]` or a singular
/// `child`, which `field` and `tooltip` use.
fn ivy_children(obj: &Map<String, Value>) -> Vec<Value> {
    if let Some(children) = obj.get("children").and_then(Value::as_array) {
        return children.iter().filter_map(to_ivy_node).collect();
    }
    // Ivy has one `children?: WidgetNode[]`, so a singular child becomes a
    // one-element array. A child that maps to None yields an empty array, not [null].
    if let Some(child) = obj.get("child") {
        return to_ivy_node(child).into_iter().collect();
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::view::{Element, WidgetData};
    use crate::widgets::button::ButtonVariant;
    use crate::widgets::text::TextVariant;
    use crate::widgets::*;

    #[test]
    fn props_are_nested_and_reserved_keys_excluded() {
        let node = to_ivy_node(&Button::new("Go").to_json()).expect("button maps");

        assert_eq!(node["props"]["title"], "Go");
        let props = node["props"].as_object().expect("props is an object");
        for excluded in ["type", "id", "children", "child", "hasOnClick"] {
            assert!(
                !props.contains_key(excluded),
                "props should not contain '{}': {:?}",
                excluded,
                props
            );
        }
        // The structural fields live at the top level, where Ivy's WidgetNode wants them.
        assert_eq!(node["type"], "Ivy.Button");
        assert!(node["id"].is_string());
        assert!(node["events"].is_array());
    }

    #[test]
    fn events_are_derived_from_has_flags() {
        let with_handler = to_ivy_node(&Button::new("Go").on_click(|| {}).to_json()).unwrap();
        assert_eq!(with_handler["events"], json!(["OnClick"]));

        // An empty array, not an absent key: Ivy does `node.events || []`, but the
        // contract should not lean on that.
        let without = to_ivy_node(&Button::new("Go").to_json()).unwrap();
        assert_eq!(without["events"], json!([]));
        assert!(without.get("events").is_some());
    }

    #[test]
    fn unreadable_events_are_dropped() {
        let terminal = Terminal::new()
            .on_input(|_| {})
            .on_resize(|_| {})
            .on_link_click(|_| {})
            .to_json();

        // Guard: the flags really are set, so an empty result means "dropped", not
        // "never emitted".
        assert_eq!(terminal["hasOnInput"], true);
        assert_eq!(terminal["hasOnResize"], true);
        assert_eq!(terminal["hasOnLinkClick"], true);

        let node = to_ivy_node(&terminal).expect("terminal maps to Ivy.Terminal");
        let events = node["events"].as_array().unwrap();
        assert!(
            !events.iter().any(|e| e == "OnResize" || e == "OnInput"),
            "OnResize/OnInput have no Ivy landing spot: {:?}",
            events
        );
        // OnLinkClick is read by Markdown/RichTextBlock, so it survives the filter
        // even though TerminalWidget itself ignores it.
        assert!(events.iter().any(|e| e == "OnLinkClick"));
    }

    #[test]
    fn expandable_on_toggle_is_dropped() {
        // hasOnToggle is new since Plan 00037, but "OnToggle" appears nowhere under
        // src/frontend/src/widgets -- ExpandableWidget takes no `events` prop.
        let expandable = Expandable::new("Details").on_toggle(|_| {}).to_json();
        assert_eq!(expandable["hasOnToggle"], true);

        let node = to_ivy_node(&expandable).expect("expandable maps to Ivy.Expandable");
        assert_eq!(node["type"], "Ivy.Expandable");
        assert_eq!(node["events"], json!([]));
    }

    #[test]
    fn danger_variant_renames_to_destructive() {
        let danger = Button::new("Delete")
            .variant(ButtonVariant::Danger)
            .to_json();
        assert_eq!(danger["variant"], "danger", "Rust still sends camelCase");
        assert_eq!(
            to_ivy_node(&danger).unwrap()["props"]["variant"],
            "Destructive"
        );

        let primary = Button::new("Save")
            .variant(ButtonVariant::Primary)
            .to_json();
        assert_eq!(
            to_ivy_node(&primary).unwrap()["props"]["variant"],
            "Primary"
        );
    }

    #[test]
    fn density_renames_onto_ivy_sizes() {
        // Rust's Density is compact/normal/comfortable; Ivy's Densities enum is
        // Small/Medium/Large, so title-casing alone would produce a non-member.
        for (rust, ivy) in [
            (crate::shared::Density::Compact, "Small"),
            (crate::shared::Density::Normal, "Medium"),
            (crate::shared::Density::Comfortable, "Large"),
        ] {
            let json = Button::new("Go").density(rust).to_json();
            assert_eq!(to_ivy_node(&json).unwrap()["props"]["density"], ivy);
        }
    }

    #[test]
    fn text_area_collapses_into_text_input() {
        let node = to_ivy_node(&TextArea::new().to_json()).expect("text_area maps");

        assert_eq!(node["type"], "Ivy.TextInput");
        assert_eq!(node["props"]["variant"], "Textarea");
    }

    #[test]
    fn injected_prop_does_not_clobber_a_sent_value() {
        // text_area does not currently send `variant`, so simulate a future version
        // that does: the widget's own value must win over the injected default.
        let mut json = TextArea::new().to_json();
        json["variant"] = Value::String("custom".to_string());

        assert_eq!(to_ivy_node(&json).unwrap()["props"]["variant"], "Custom");
    }

    #[test]
    fn singular_child_becomes_a_children_array() {
        let tooltip = Tooltip::new("tip", TextBlock::new("hi")).to_json();
        assert!(tooltip.get("child").is_some(), "tooltip serializes `child`");
        assert!(tooltip.get("children").is_none());

        let node = to_ivy_node(&tooltip).unwrap();
        let children = node["children"].as_array().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0]["type"], "Ivy.TextBlock");
        // `child` must not survive as a prop.
        assert!(node["props"].get("child").is_none());

        let field = Field::new("Label", TextBlock::new("hi")).to_json();
        let field_node = to_ivy_node(&field).unwrap();
        assert_eq!(field_node["children"].as_array().unwrap().len(), 1);
        assert_eq!(field_node["children"][0]["type"], "Ivy.TextBlock");
        assert_eq!(field_node["props"]["label"], "Label");
    }

    #[test]
    fn rust_only_widgets_yield_none() {
        assert!(to_ivy_node(&QrCode::new("x").to_json()).is_none());
        assert!(to_ivy_node(&DiffView::new().to_json()).is_none());
        assert!(to_ivy_node(&ActivityHeatmap::new().to_json()).is_none());
        assert!(to_ivy_node(&RichTextInput::new().to_json()).is_none());
        // Backfilled as RustOnly in this plan.
        assert!(to_ivy_node(&MultiSelect::new(vec![]).to_json()).is_none());
        assert!(to_ivy_node(&RadioGroup::new(vec![]).to_json()).is_none());
        assert!(to_ivy_node(&Slider::new(0.0).to_json()).is_none());
    }

    #[test]
    fn rust_only_children_are_dropped_not_nulled() {
        let card = Card::new()
            .child(QrCode::new("x"))
            .child(TextBlock::new("keep"))
            .to_json();
        assert_eq!(card["children"].as_array().unwrap().len(), 2);

        let children = to_ivy_node(&card).unwrap()["children"].clone();
        let children = children.as_array().unwrap();
        assert_eq!(children.len(), 1, "the QrCode child should be dropped");
        assert!(!children.iter().any(Value::is_null));
        assert_eq!(children[0]["props"]["content"], "keep");
    }

    #[test]
    fn non_widget_children_are_dropped() {
        // Element::Empty serializes as {"kind":"empty"} with no `type` at all.
        let card = Card::new()
            .child(Element::Empty)
            .child(TextBlock::new("keep"))
            .to_json();

        let node = to_ivy_node(&card).unwrap();
        let children = node["children"].as_array().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0]["type"], "Ivy.TextBlock");
    }

    #[test]
    fn element_kind_tag_does_not_leak_into_props() {
        // Element is #[serde(tag = "kind")], so a widget reached as a *child* carries
        // "kind": "widget" merged in beside its real props. That describes Rusty's
        // element tree, not the widget, and Ivy has no such field -- it must not
        // survive into props. A top-level widget's own to_json() has no `kind`, so this
        // only shows up one level down.
        let card = Card::new().child(TextBlock::new("hi")).to_json();
        assert_eq!(
            card["children"][0]["kind"], "widget",
            "Element's tag really is present in the input"
        );

        let node = to_ivy_node(&card).unwrap();
        let child_props = node["children"][0]["props"].as_object().unwrap();
        assert!(
            !child_props.contains_key("kind"),
            "Element's serde tag leaked into props: {:?}",
            child_props
        );
        // The real props are still there.
        assert_eq!(child_props["content"], "hi");

        // Same for the singular-child path.
        let tooltip = Tooltip::new("tip", TextBlock::new("hi")).to_json();
        let tooltip_node = to_ivy_node(&tooltip).unwrap();
        assert!(!tooltip_node["children"][0]["props"]
            .as_object()
            .unwrap()
            .contains_key("kind"));
    }

    #[test]
    fn layout_children_recurse_and_resolve_by_direction() {
        let grid = Layout::grid(2)
            .child(TextBlock::new("a"))
            .child(Button::new("b"))
            .to_json();

        let node = to_ivy_node(&grid).expect("layout maps by direction");
        assert_eq!(node["type"], "Ivy.GridLayout");
        assert_eq!(node["props"]["columns"], 2);

        let children = node["children"].as_array().unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0]["type"], "Ivy.TextBlock");
        assert_eq!(children[1]["type"], "Ivy.Button");

        // The other arm of the same mapping.
        let stack = Layout::vertical().child(TextBlock::new("a")).to_json();
        assert_eq!(to_ivy_node(&stack).unwrap()["type"], "Ivy.StackLayout");
    }

    #[test]
    fn nesting_recurses_to_arbitrary_depth() {
        let tree = Card::new()
            .child(
                Layout::vertical().child(Tooltip::new("tip", Button::new("deep").on_click(|| {}))),
            )
            .to_json();

        let node = to_ivy_node(&tree).unwrap();
        let button = &node["children"][0]["children"][0]["children"][0];
        assert_eq!(button["type"], "Ivy.Button");
        assert_eq!(button["props"]["title"], "deep");
        assert_eq!(button["events"], json!(["OnClick"]));
    }

    #[test]
    fn user_text_is_not_title_cased() {
        let node = to_ivy_node(&TextBlock::new("hello world").to_json()).unwrap();
        assert_eq!(node["props"]["content"], "hello world");

        // `title` on a Button is user text too, despite living beside enum props.
        let button = to_ivy_node(&Button::new("save changes").to_json()).unwrap();
        assert_eq!(button["props"]["title"], "save changes");
    }

    #[test]
    fn text_block_variants_map_onto_ivy_vocabulary() {
        // Ivy's TextBlockVariant is a different vocabulary, not a recasing of Rust's.
        for (rust, ivy) in [
            (TextVariant::Block, "Block"),
            (TextVariant::Heading1, "H1"),
            (TextVariant::Heading4, "H4"),
            (TextVariant::Paragraph, "P"),
            (TextVariant::Code, "Monospaced"),
            (TextVariant::Markdown, "Lead"),
            (TextVariant::Label, "Label"),
            (TextVariant::Caption, "Muted"),
        ] {
            let json = TextBlock::new("x").variant(rust).to_json();
            assert_eq!(
                to_ivy_node(&json).unwrap()["props"]["variant"],
                ivy,
                "{:?} should map to {}",
                rust,
                ivy
            );
        }
    }

    #[test]
    fn non_string_props_survive_untouched() {
        let json = Terminal::new().cols(80).rows(24).to_json();
        let props = &to_ivy_node(&json).unwrap()["props"];

        assert_eq!(props["cols"], 80);
        assert_eq!(props["rows"], 24);
        assert_eq!(props["closed"], false);
        // A null prop stays null rather than becoming "Null" via the enum path.
        assert!(props["background"].is_null());
    }

    #[test]
    fn hex_color_is_not_recased() {
        // Color is an untagged enum: Named recases, but a hex string is a literal.
        let json = Badge::new("x")
            .color(crate::shared::Color::hex("#ff0000"))
            .to_json();
        assert_eq!(to_ivy_node(&json).unwrap()["props"]["color"], "#ff0000");

        let named = Badge::new("x")
            .color(crate::shared::Color::Named(
                crate::shared::NamedColor::Danger,
            ))
            .to_json();
        assert_eq!(to_ivy_node(&named).unwrap()["props"]["color"], "Danger");
    }

    #[test]
    fn malformed_input_returns_none() {
        assert!(to_ivy_node(&json!(null)).is_none());
        assert!(to_ivy_node(&json!("button")).is_none());
        assert!(to_ivy_node(&json!([])).is_none());
        assert!(to_ivy_node(&json!({})).is_none(), "no type key");
        assert!(to_ivy_node(&json!({ "type": 7 })).is_none(), "type not str");
        assert!(to_ivy_node(&json!({ "type": "not_a_widget" })).is_none());

        // ivy_events is total: it never panics on a non-object.
        assert!(ivy_events(&json!(null)).is_empty());
        assert!(ivy_events(&json!("x")).is_empty());
    }

    #[test]
    fn events_are_sorted_and_only_true_flags_count() {
        let json = json!({
            "type": "text_input",
            "hasOnSubmit": true,
            "hasOnChange": true,
            "hasOnBlur": true,
            "hasOnFocus": false,
        });

        // Sorted for determinism, and hasOnFocus: false contributes nothing.
        assert_eq!(ivy_events(&json), vec!["OnBlur", "OnChange", "OnSubmit"]);
    }

    #[test]
    fn assigned_ids_reach_the_node() {
        let mut button = Button::new("Go");
        button.assign_id("w42".to_string());

        let node = to_ivy_node(&button.to_json()).unwrap();
        assert_eq!(node["id"], "w42");
        // Ivy re-injects id into props itself; do not duplicate it there.
        assert!(node["props"].get("id").is_none());
    }
}
