//! Tests for the `#[derive(Widget)]` attribute surface.
//!
//! These live in `rusty` rather than in `rusty-macros` because the macro crate
//! compiles first and cannot name `WidgetData`, `Element` or the widgets the
//! generated code refers to. They assert through the two seams the runtime
//! itself uses — `WidgetData::to_json()` and the `EventRegistry` — rather than
//! on expanded tokens, so they pin behaviour instead of formatting.

use crate::core::event_registry::EventRegistry;
use crate::hooks::hook_store::HookStore;
use crate::shared::Size;
use crate::views::view::{BuildContext, Element, WidgetData};
use crate::widgets::{
    Button, Card, ColType, DataTable, DataTableColumn, IconWidget, List, RowActionArgs, Skeleton,
    Terminal, TerminalSize, Tooltip,
};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Assign ids to a tree and hand back the walked element plus the registry the
/// walk populated, mirroring what the runtime does after a build.
fn assign(element: impl Into<Element>) -> (Element, EventRegistry) {
    let mut store = HookStore::default();
    let mut ctx = BuildContext::new(&mut store, None);
    let mut element: Element = element.into();
    element.assign_ids(&mut ctx);
    let registry = ctx.take_event_registry();
    (element, registry)
}

fn json_of(element: &Element) -> serde_json::Value {
    match element {
        Element::Widget(widget) => widget.to_json(),
        _ => panic!("Expected Element::Widget"),
    }
}

// 1. register_events fires for each EventArg shape.

#[test]
fn test_derived_register_events_covers_every_event_arg_shape() {
    let input = Arc::new(Mutex::new(None::<String>));
    let resize = Arc::new(Mutex::new(None::<TerminalSize>));
    let link = Arc::new(Mutex::new(None::<String>));

    let (input_clone, resize_clone, link_clone) = (input.clone(), resize.clone(), link.clone());
    let terminal = Terminal::new()
        .on_input(move |data| *input_clone.lock().unwrap() = Some(data))
        .on_resize(move |size| *resize_clone.lock().unwrap() = Some(size))
        .on_link_click(move |url| *link_clone.lock().unwrap() = Some(url));

    let (_, registry) = assign(terminal);

    // EventArg::Key("data") — the handler takes the value under one key.
    assert!(registry.dispatch("w-0", "input", json!({"data": "echo hi\r"})));
    assert_eq!(input.lock().unwrap().as_deref(), Some("echo hi\r"));

    // EventArg::Payload — the handler takes the whole args object.
    assert!(registry.dispatch("w-0", "resize", json!({"cols": 100, "rows": 30})));
    assert_eq!(
        *resize.lock().unwrap(),
        Some(TerminalSize {
            cols: 100,
            rows: 30
        })
    );

    // EventArg::Key("url") — a second keyed arm on the same struct.
    assert!(registry.dispatch("w-0", "linkclick", json!({"url": "https://example.com"})));
    assert_eq!(link.lock().unwrap().as_deref(), Some("https://example.com"));

    // EventArg::None — the payload is ignored entirely.
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_clone = hits.clone();
    let (_, registry) = assign(Button::new("Go").on_click(move || {
        hits_clone.fetch_add(1, Ordering::SeqCst);
    }));
    assert!(registry.dispatch("w-0", "click", serde_json::Value::Null));
    assert!(registry.dispatch("w-0", "click", json!({"unrelated": 1})));
    assert_eq!(hits.load(Ordering::SeqCst), 2);
}

#[test]
fn test_derived_event_name_matches_the_registry_canonical_form() {
    // `on_link_click` -> `linkclick`, which the registry also reaches from the
    // camelCase form the browser sends.
    let (_, registry) = assign(Terminal::new().on_link_click(|_| {}));
    assert!(registry.dispatch("w-0", "linkclick", json!({"url": "u"})));
    assert!(registry.dispatch("w-0", "onLinkClick", json!({"url": "u"})));
    assert!(!registry.dispatch("w-0", "link_click", json!({"url": "u"})));
}

// 2. Malformed payloads are dropped, not panicked on.

#[test]
fn test_derived_keyed_handler_drops_malformed_payloads() {
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_clone = hits.clone();
    let (_, registry) = assign(Terminal::new().on_input(move |_| {
        hits_clone.fetch_add(1, Ordering::SeqCst);
    }));

    // A handler is registered, so dispatch reports a hit either way; what must
    // not happen is the handler running on a payload it cannot decode.
    assert!(registry.dispatch("w-0", "input", serde_json::Value::Null));
    assert!(registry.dispatch("w-0", "input", json!({})));
    assert!(registry.dispatch("w-0", "input", json!({"data": 42})));
    assert!(registry.dispatch("w-0", "input", json!({"data": null})));
    assert!(registry.dispatch("w-0", "input", json!({"wrongKey": "hi"})));
    assert_eq!(hits.load(Ordering::SeqCst), 0);

    // ...and a well-formed payload still gets through afterwards.
    assert!(registry.dispatch("w-0", "input", json!({"data": "ok"})));
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[test]
fn test_derived_payload_handler_drops_malformed_payloads() {
    let seen: Arc<Mutex<Vec<RowActionArgs>>> = Arc::new(Mutex::new(Vec::new()));
    let seen_clone = seen.clone();
    let columns = vec![DataTableColumn::new("name", "Name", ColType::Text)];
    let (_, registry) = assign(
        DataTable::new(columns).on_row_action(move |args| seen_clone.lock().unwrap().push(args)),
    );

    // `RowActionArgs` is all-`Option` and `#[serde(default)]`, so an empty
    // object decodes; a non-object or a wrong-typed field cannot.
    assert!(registry.dispatch("w-0", "rowaction", json!("not an object")));
    assert!(registry.dispatch("w-0", "rowaction", json!([1, 2])));
    assert!(registry.dispatch("w-0", "rowaction", json!({"tag": 7})));
    assert!(seen.lock().unwrap().is_empty());

    assert!(registry.dispatch("w-0", "rowaction", json!({"tag": "delete"})));
    assert_eq!(seen.lock().unwrap().len(), 1);
    assert_eq!(seen.lock().unwrap()[0].tag.as_deref(), Some("delete"));
}

// 3. #[children] / #[child] / #[footer] reach descendants.
//
// Each nests a Button one level down and asserts the walk both assigned it an
// id and registered its click — the regression test for the derive having
// hardcoded a container field named `children`.

/// Walk `path` through a widget's JSON and assert the node there is a Button
/// that `assign_ids` reached.
fn assert_button_at(element: &Element, path: &[&str], expected_id: &str) {
    let mut node = json_of(element);
    for key in path {
        node = match key.parse::<usize>() {
            Ok(index) => node[index].clone(),
            Err(_) => node[*key].clone(),
        };
    }
    assert_eq!(node["type"], "button");
    assert_eq!(node["id"], expected_id, "nested button did not get an id");
    assert_eq!(node["hasOnClick"], true);
}

#[test]
fn test_derived_children_attribute_reaches_descendants() {
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_clone = hits.clone();

    // `List`'s container field is `items`, which the pre-attribute derive could
    // not see at all.
    let (element, registry) = assign(List::new().item(Button::new("Inner").on_click(move || {
        hits_clone.fetch_add(1, Ordering::SeqCst);
    })));

    assert_button_at(&element, &["items", "0"], "w-1");
    assert!(registry.dispatch("w-1", "click", serde_json::Value::Null));
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[test]
fn test_derived_child_attribute_reaches_descendant() {
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_clone = hits.clone();

    let (element, registry) = assign(Tooltip::new(
        "Help",
        Button::new("Inner").on_click(move || {
            hits_clone.fetch_add(1, Ordering::SeqCst);
        }),
    ));

    assert_button_at(&element, &["child"], "w-1");
    assert!(registry.dispatch("w-1", "click", serde_json::Value::Null));
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[test]
fn test_derived_footer_attribute_reaches_descendants() {
    let body_hits = Arc::new(AtomicUsize::new(0));
    let footer_hits = Arc::new(AtomicUsize::new(0));
    let (body_clone, footer_clone) = (body_hits.clone(), footer_hits.clone());

    // Card carries `#[children]` and `#[footer]` on one struct, and its footer
    // is an `Option<Vec<Element>>` rather than a bare `Vec`.
    let (element, registry) = assign(
        Card::new()
            .child(Button::new("Body").on_click(move || {
                body_clone.fetch_add(1, Ordering::SeqCst);
            }))
            .footer(vec![Button::new("Foot")
                .on_click(move || {
                    footer_clone.fetch_add(1, Ordering::SeqCst);
                })
                .into()]),
    );

    assert_button_at(&element, &["children", "0"], "w-1");
    assert_button_at(&element, &["footer", "0"], "w-2");
    assert!(registry.dispatch("w-1", "click", serde_json::Value::Null));
    assert!(registry.dispatch("w-2", "click", serde_json::Value::Null));
    assert_eq!(body_hits.load(Ordering::SeqCst), 1);
    assert_eq!(footer_hits.load(Ordering::SeqCst), 1);
}

#[test]
fn test_derived_absent_footer_is_not_walked() {
    // `footer_mut` on a `None` footer must yield None, or the id counter would
    // drift for anything nested deeper.
    let (element, _) = assign(Card::new().child(Button::new("Body")));
    let json = json_of(&element);
    assert!(json["footer"].is_null());
    assert_eq!(json["children"][0]["id"], "w-1");
}

// 4. #[widget(type = "...")] override.

#[test]
fn test_derived_widget_type_override_wins_over_the_struct_name() {
    // Snake-casing `IconWidget` would give "icon_widget"; the frontend registry
    // keys on "icon".
    assert_eq!(IconWidget::new("star").widget_type(), "icon");
    assert_eq!(IconWidget::new("star").to_json()["type"], "icon");
}

#[test]
fn test_derived_widget_type_defaults_to_the_snake_case_struct_name() {
    assert_eq!(Skeleton::new().widget_type(), "skeleton");
    assert_eq!(DataTable::new(vec![]).widget_type(), "data_table");
    assert_eq!(Skeleton::new().to_json()["type"], "skeleton");
}

// 5. #[prop(with = ...)] keeps Size lossless.

#[test]
fn test_derived_prop_with_keeps_size_lossless() {
    // `Size`'s derived Serialize is untagged, so Px(200.0) and Percent(200.0)
    // both reach the wire as the bare number 200.0. The hook emits CSS instead.
    let json = Skeleton::new()
        .width(Size::Px(200.0))
        .height(Size::Percent(30.0))
        .to_json();
    assert_eq!(json["width"], "200px");
    assert_eq!(json["height"], "30%");

    // The two values a plain `#[prop]` would collapse into one.
    assert_eq!(
        Skeleton::new().width(Size::Px(200.0)).to_json()["width"],
        "200px"
    );
    assert_eq!(
        Skeleton::new().width(Size::Percent(200.0)).to_json()["width"],
        "200%"
    );
    assert_eq!(Skeleton::new().width(Size::Auto).to_json()["width"], "auto");
}

#[test]
fn test_derived_prop_with_omits_an_unset_size() {
    // `size_css` maps None to None, so an unset size stays null rather than
    // becoming "auto" or the string "null".
    let json = Skeleton::new().to_json();
    assert!(json["width"].is_null());
    assert!(json["height"].is_null());
}

// Cross-cutting: the has<Event> booleans, and the no-event case.

#[test]
fn test_derived_has_event_booleans_track_the_option() {
    assert_eq!(Button::new("x").to_json()["hasOnClick"], false);
    assert_eq!(
        Button::new("x").on_click(|| {}).to_json()["hasOnClick"],
        true
    );

    let json = Terminal::new().on_resize(|_| {}).to_json();
    assert_eq!(json["hasOnInput"], false);
    assert_eq!(json["hasOnResize"], true);
    assert_eq!(json["hasOnLinkClick"], false);
}

#[test]
fn test_derived_widget_without_events_registers_nothing() {
    // No `#[event]` field means no generated `register_events`, so the trait's
    // no-op default applies and nothing lands in the registry.
    let (_, registry) = assign(Skeleton::new().width(Size::Px(10.0)));
    assert!(!registry.dispatch("w-0", "click", serde_json::Value::Null));
}
