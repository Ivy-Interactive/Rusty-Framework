//! Tests for the `ivyml!` markup macro.
//!
//! These live in `rusty` rather than in `rusty-ivyml` because a `proc-macro`
//! crate exports only macros and cannot expand them against `rusty`'s widgets,
//! so there is nothing to assert from inside it.
//!
//! The first and last tests are *equivalence* tests rather than shape tests:
//! they compare the markup's serialized output against the hand-written builder
//! chain a reviewer already trusts. If lowering drifts, the JSON stops matching.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use rusty::hooks::hook_store::HookStore;
use rusty::ivyml;
use rusty::shared::{Justify, Size};
use rusty::views::view::{BuildContext, Element};
use rusty::widgets::text::TextVariant;
use rusty::widgets::{Card, Layout, List, ListItem, TextBlock};

/// Serialize an element tree the way the server sends it to the client.
fn json(element: &Element) -> serde_json::Value {
    serde_json::to_value(element).expect("element serializes")
}

/// Assign widget IDs so `id` fields are populated and handlers registered.
fn assign_ids(element: &mut Element) -> rusty::core::event_registry::EventRegistry {
    let mut store = HookStore::default();
    let mut ctx = BuildContext::new(&mut store, None);
    element.assign_ids(&mut ctx);
    ctx.take_event_registry()
}

#[test]
fn markup_matches_the_equivalent_builder_chain() {
    let from_markup: Element = ivyml! {
        <Layout direction="vertical" gap=16 padding=24>
            <TextBlock content="Hello, World!" variant="heading1" />
            <TextBlock content="This is a Rusty-Framework application." variant="paragraph" />
        </Layout>
    };

    let from_builder: Element = Layout::vertical()
        .gap(16.0)
        .padding(24.0)
        .child(TextBlock::new("Hello, World!").variant(TextVariant::Heading1))
        .child(
            TextBlock::new("This is a Rusty-Framework application.")
                .variant(TextVariant::Paragraph),
        )
        .into();

    assert_eq!(json(&from_markup), json(&from_builder));
}

#[test]
fn nested_containers_keep_their_child_order() {
    let element: Element = ivyml! {
        <Layout direction="vertical">
            <TextBlock content="first" />
            <Card>
                <TextBlock content="inner-a" />
                <TextBlock content="inner-b" />
            </Card>
            <TextBlock content="last" />
        </Layout>
    };

    let value = json(&element);
    let children = &value["children"];
    assert_eq!(children[0]["content"], "first");
    assert_eq!(children[1]["type"], "card");
    assert_eq!(children[1]["children"][0]["content"], "inner-a");
    assert_eq!(children[1]["children"][1]["content"], "inner-b");
    assert_eq!(children[2]["content"], "last");
}

#[test]
fn list_children_use_the_item_builder_not_child() {
    // `List` stores `items` and has no `.child` method at all, so this is the
    // test that pins `child_method` as per-element rather than always `.child`.
    let element: Element = ivyml! {
        <List>
            <ListItem title="one" />
            <ListItem title="two" subtitle="second" />
        </List>
    };

    let value = json(&element);
    assert_eq!(value["type"], "list");
    assert!(
        value.get("children").is_none(),
        "list has items, not children"
    );
    assert_eq!(value["items"][0]["title"], "one");
    assert_eq!(value["items"][1]["title"], "two");
    assert_eq!(value["items"][1]["subtitle"], "second");

    let from_builder: Element = List::new()
        .item(ListItem::new("one"))
        .item(ListItem::new("two").subtitle("second"))
        .into();
    assert_eq!(value, json(&from_builder));
}

#[test]
fn size_literals_reach_the_wire_as_css_not_bare_numbers() {
    // `Size` derives `#[serde(untagged)]`, so `Px(240.0)` and `Percent(240.0)`
    // both serialize to a bare `240.0`; the widgets emit `to_css()` by hand.
    // Asserting the CSS strings is what proves the right variant was chosen.
    let element: Element = ivyml! {
        <Layout direction="horizontal" width="100%" height="240px" />
    };

    let value = json(&element);
    assert_eq!(value["width"], "100%");
    assert_eq!(value["height"], "240px");

    let from_builder: Element = Layout::horizontal()
        .width(Size::Percent(100.0))
        .height(Size::Px(240.0))
        .into();
    assert_eq!(value, json(&from_builder));

    // `auto` is the third variant, and it serializes to `null` without `to_css`.
    let auto: Element = ivyml! { <Layout width="auto" /> };
    assert_eq!(json(&auto)["width"], "auto");
}

#[test]
fn grid_direction_passes_columns_to_the_constructor() {
    let element: Element = ivyml! {
        <Layout direction="grid" columns=3 gap=8>
            <TextBlock content="cell" />
        </Layout>
    };

    let value = json(&element);
    assert_eq!(value["direction"], "grid");
    assert_eq!(value["columns"], 3);
    assert_eq!(value["gap"], 8.0);

    let from_builder: Element = Layout::grid(3)
        .gap(8.0)
        .child(TextBlock::new("cell"))
        .into();
    assert_eq!(value, json(&from_builder));
}

#[test]
fn interpolated_string_expressions_coerce_into_str_slots() {
    // Most Rusty constructors take `&str`, and the common case in a real view is
    // a `format!`, which yields `String`. Lowering emits `&(expr)` so deref
    // coercion covers `String`, `&String`, `&str` and `format!(..)` alike.
    let n = 7;
    let owned: String = "owned".to_string();
    let borrowed: &String = &owned;
    let slice: &str = "slice";

    let element: Element = ivyml! {
        <Layout direction="vertical">
            <TextBlock content={format!("count = {}", n)} />
            <TextBlock content={owned.clone()} />
            <TextBlock content={borrowed} />
            <TextBlock content={slice} />
        </Layout>
    };

    let children = &json(&element)["children"];
    assert_eq!(children[0]["content"], "count = 7");
    assert_eq!(children[1]["content"], "owned");
    assert_eq!(children[2]["content"], "owned");
    assert_eq!(children[3]["content"], "slice");
}

#[test]
fn interpolated_element_expressions_splice_into_child_position() {
    let existing: Element = TextBlock::new("spliced").into();
    let widget = Card::new().child(TextBlock::new("from-builder"));

    let element: Element = ivyml! {
        <Layout direction="vertical">
            <TextBlock content="literal" />
            {existing}
            {widget}
        </Layout>
    };

    let children = &json(&element)["children"];
    assert_eq!(children[0]["content"], "literal");
    assert_eq!(children[1]["content"], "spliced");
    assert_eq!(children[2]["type"], "card");
    assert_eq!(children[2]["children"][0]["content"], "from-builder");
}

#[test]
fn handlers_register_and_dispatch_after_assign_ids() {
    // The test that pins `on_*` as its own argument class. With handlers falling
    // through to the `&str` default the closure is passed as `&(..)`, which is a
    // borrowed temporary and cannot satisfy `on_click`'s `'static` bound (E0716)
    // — so this would not compile at all rather than fail an assertion.
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_clone = hits.clone();

    let mut element: Element = ivyml! {
        <Button title="Click me" on_click={move || { hits_clone.fetch_add(1, Ordering::SeqCst); }} />
    };

    let registry = assign_ids(&mut element);
    assert_eq!(json(&element)["hasOnClick"], true);
    assert!(registry.dispatch("w-0", "click", serde_json::Value::Null));
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[test]
fn enum_valued_attributes_accept_kebab_and_snake_spelling() {
    let kebab: Element = ivyml! {
        <Layout direction="horizontal" justify="space-between" align="center" />
    };
    let snake: Element = ivyml! {
        <Layout direction="horizontal" justify="space_between" align="center" />
    };

    let value = json(&kebab);
    assert_eq!(value["justify"], "spaceBetween");
    assert_eq!(value["align"], "center");
    assert_eq!(value, json(&snake));

    let from_builder: Element = Layout::horizontal()
        .justify(Justify::SpaceBetween)
        .align(rusty::shared::Align::Center)
        .into();
    assert_eq!(value, json(&from_builder));

    // Per-widget `variant` enums resolve against the element, not one shared enum.
    let button: Element = ivyml! { <Button title="x" variant="ghost" /> };
    assert_eq!(json(&button)["variant"], "ghost");
    let text: Element = ivyml! { <TextBlock content="x" variant="heading1" /> };
    assert_eq!(json(&text)["variant"], "heading1");
}

#[test]
fn self_closing_and_paired_forms_are_equivalent() {
    let self_closing: Element = ivyml! { <Card /> };
    let paired: Element = ivyml! { <Card></Card> };
    assert_eq!(json(&self_closing), json(&paired));
    assert_eq!(json(&self_closing), json(&Card::new().into()));

    // And the same for an element that does carry attributes.
    let a: Element = ivyml! { <Layout direction="vertical" gap=4 /> };
    let b: Element = ivyml! { <Layout direction="vertical" gap=4></Layout> };
    assert_eq!(json(&a), json(&b));
}
