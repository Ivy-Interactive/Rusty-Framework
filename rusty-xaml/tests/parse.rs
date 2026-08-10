//! The crate from the outside: whole documents, whole trees.
//!
//! The unit tests in `src/build.rs` cover one element at a time. These cover the
//! properties that only show up once several elements are nested — that a parsed
//! tree is indistinguishable from a hand-built one, that ids and events survive
//! the trip through `assign_ids`, and that the vocabulary has not fallen behind
//! `rusty-ivyml`'s.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use rusty::hooks::hook_store::HookStore;
use rusty::views::{BuildContext, Element};
use rusty::widgets::text::TextVariant;
use rusty::widgets::{Badge, Button, Card, Container, Layout, List, ListItem, TextBlock};
use rusty_xaml::{parse, parse_file, parse_with, XamlContext, XamlError};
use serde_json::Value;

fn json(element: &Element) -> Value {
    serde_json::to_value(element).expect("a widget tree serializes")
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn a_nested_document_matches_the_equivalent_builder_chain() {
    let parsed = parse(
        r#"<Grid Columns="2" Spacing="16" Padding="24">
               <StackPanel Orientation="Horizontal" Spacing="8" HorizontalAlignment="Center">
                   <Border Padding="12" Rounded="True">
                       <TextBlock Text="Revenue" Variant="Heading2" />
                   </Border>
                   <Button Content="Refresh" Variant="Secondary" />
               </StackPanel>
           </Grid>"#,
    )
    .expect("parses");

    let built: Element = Layout::grid(2)
        .gap(16.0)
        .padding(24.0)
        .child(
            Layout::horizontal()
                .gap(8.0)
                .align(rusty::shared::Align::Center)
                .child(
                    Container::new()
                        .padding(12.0)
                        .rounded(true)
                        .child(TextBlock::new("Revenue").variant(TextVariant::Heading2)),
                )
                .child(
                    Button::new("Refresh")
                        .variant(rusty::widgets::button::ButtonVariant::Secondary),
                ),
        )
        .into();

    assert_eq!(json(&parsed), json(&built));
}

#[test]
fn nesting_three_deep_mixes_child_and_item_attachment() {
    let parsed = parse(
        r#"<Card Header="Invoices">
               <List>
                   <ListItem Content="Invoice #1041" Subtitle="Paid" />
                   <Container>
                       <Badge Content="2 overdue" Variant="Dot" />
                   </Container>
               </List>
           </Card>"#,
    )
    .expect("parses");

    let built: Element = Card::new()
        .title("Invoices")
        .child(
            List::new()
                .item(ListItem::new("Invoice #1041").subtitle("Paid"))
                .item(Container::new().child(
                    Badge::new("2 overdue").variant(rusty::widgets::badge::BadgeVariant::Dot),
                )),
        )
        .into();

    let wire = json(&parsed);
    assert_eq!(wire, json(&built));

    // The middle level attached with `item`, not `child`: a `List` has no
    // `children` at all, and getting this wrong renders an empty list.
    assert_eq!(wire["children"][0]["type"], "list");
    assert_eq!(wire["children"][0]["items"][0]["title"], "Invoice #1041");
    assert!(wire["children"][0]["children"].is_null());
    assert_eq!(
        wire["children"][0]["items"][1]["children"][0]["type"],
        "badge"
    );
}

#[test]
fn parse_file_matches_parsing_the_same_text() {
    let path = fixture("dashboard.xaml");
    let text = std::fs::read_to_string(&path).expect("the fixture is readable");

    let from_file = parse_file(&path).expect("the fixture parses");
    let from_text = parse(&text).expect("the fixture text parses");

    assert_eq!(json(&from_file), json(&from_text));

    // A spot check that it is the document, not just two equal errors.
    let wire = json(&from_file);
    assert_eq!(wire["type"], "layout");
    assert_eq!(wire["children"][0]["content"], "Dashboard");
    assert_eq!(wire["children"][1]["columns"], 2);
}

#[test]
fn a_missing_file_names_the_path_it_tried() {
    let path = fixture("does-not-exist.xaml");
    let err = parse_file(&path).expect_err("a missing file is an error");

    assert!(matches!(err, XamlError::Io { .. }), "{err:?}");
    assert!(
        err.to_string().contains("does-not-exist.xaml"),
        "{}",
        err.to_string()
    );

    // The cause is preserved, so a caller can distinguish "missing" from
    // "unreadable".
    let source = std::error::Error::source(&err).expect("the io error is the cause");
    assert!(source.to_string().to_lowercase().contains("no such file"));
}

#[test]
fn document_furniture_does_not_change_the_tree() {
    let bare = parse(r#"<StackPanel Spacing="8"><TextBlock Text="Hi" /></StackPanel>"#).unwrap();
    let decorated = parse(
        r#"<?xml version="1.0" encoding="utf-8"?>
           <!-- a leading comment -->
           <StackPanel xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
                       xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
                       x:Name="Root"
                       Spacing="8">
               <!-- an inner comment -->
               <TextBlock x:Name="Greeting" Text="Hi" />
           </StackPanel>"#,
    )
    .unwrap();

    assert_eq!(json(&bare), json(&decorated));
}

#[test]
fn ids_and_events_survive_assign_ids() {
    let clicks = Arc::new(AtomicUsize::new(0));
    let on_refresh = clicks.clone();
    let on_pick = clicks.clone();
    let ctx = XamlContext::new()
        .value("Heading", "Invoices")
        .handler("OnRefresh", move || {
            on_refresh.fetch_add(1, Ordering::SeqCst);
        })
        .handler("OnPick", move || {
            on_pick.fetch_add(1, Ordering::SeqCst);
        });

    let mut element = parse_with(
        r#"<StackPanel>
               <TextBlock Text="{Binding Heading}" />
               <Button Content="Refresh" Click="OnRefresh" />
               <List>
                   <ListItem Content="Invoice #1041" Click="OnPick" />
               </List>
           </StackPanel>"#,
        &ctx,
    )
    .expect("parses");

    let mut store = HookStore::default();
    let mut build = BuildContext::new(&mut store, None);
    element.assign_ids(&mut build);

    let wire = json(&element);
    let clickable = clickable_ids(&wire);
    assert_eq!(clickable.len(), 2, "{:#?}", clickable);

    for (id, _) in &clickable {
        assert!(
            build
                .event_registry_mut()
                .dispatch(id, "click", Value::Null),
            "no handler registered for {id}"
        );
    }

    assert_eq!(clicks.load(Ordering::SeqCst), 2);

    // Every widget was given an id, including the ones with no handler.
    assert!(ids(&wire).len() >= 5, "{:#?}", ids(&wire));
}

/// Every element `rusty-ivyml` accepts at compile time is also accepted here.
///
/// A drift guard rather than a list: `codegen.rs` is the compile-time
/// vocabulary, and an element added there but not here would silently mean
/// "IvyML can express this, XAML cannot". The table below is the claim being
/// checked — that each name has markup this crate can parse — so adding a name
/// upstream fails this test until a mapping exists.
#[test]
fn mapping_covers_every_ivyml_element() {
    // Minimal markup per IvyML element, using its Rusty name so the alias path
    // is what gets exercised.
    let mapping: BTreeMap<&str, &str> = BTreeMap::from([
        ("Badge", r#"<Badge Content="New" />"#),
        ("Button", r#"<Button Content="Go" />"#),
        ("Card", r#"<Card />"#),
        ("Container", r#"<Container />"#),
        ("Layout", r#"<Layout />"#),
        ("List", r#"<List />"#),
        ("ListItem", r#"<ListItem Content="One" />"#),
        ("Spacer", r#"<Spacer />"#),
        ("TextBlock", r#"<TextBlock Text="Hi" />"#),
        ("TextInput", r#"<TextInput />"#),
    ]);

    for name in ivyml_element_names() {
        let markup = mapping.get(name.as_str()).unwrap_or_else(|| {
            panic!(
                "`<{name}>` is an IvyML element with no XAML mapping. Add it to \
                 `Build::element` in `rusty-xaml/src/build.rs`, then to this table."
            )
        });

        let element = parse(markup)
            .unwrap_or_else(|err| panic!("the mapping for `<{name}>` does not parse: {err}"));
        assert_eq!(
            json(&element)["kind"],
            "widget",
            "the mapping for `<{name}>` did not produce a widget"
        );
    }
}

/// The element names in `rusty-ivyml`'s `shape_for`.
///
/// Read from source rather than imported: `shape_for` is private, and the arms
/// are string literals in a `match`, so there is nothing to import even if it
/// were public.
fn ivyml_element_names() -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../rusty-ivyml/src/codegen.rs");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()));

    let body = source
        .split_once("fn shape_for(")
        .expect("`shape_for` still exists")
        .1;
    // The leading newline matters: `shape_for`'s nested `match` has a catch-all of
    // its own, deeper indented, and without it the scan stops at the first one.
    let body = body
        .split_once("\n        other => {")
        .expect("`shape_for` still ends in a catch-all arm")
        .0;

    // The arms of `shape_for`'s own `match` are indented by exactly eight spaces;
    // the nested `match` on `direction` is deeper, which is what keeps
    // `"vertical"` out of this list.
    let names: Vec<String> = body
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("        \"")?;
            let (name, tail) = rest.split_once('"')?;
            tail.trim_start()
                .starts_with("=>")
                .then(|| name.to_string())
        })
        .collect();

    assert!(
        names.len() >= 10,
        "only found {} IvyML elements, so the scan is probably broken: {:?}",
        names.len(),
        names
    );

    names
}

/// Every `id` in a serialized tree.
fn ids(wire: &Value) -> Vec<String> {
    let mut found = Vec::new();
    walk(wire, &mut |node| {
        if let Some(id) = node.get("id").and_then(Value::as_str) {
            found.push(id.to_string());
        }
    });
    found
}

/// The `id` of every widget that reported a click handler.
fn clickable_ids(wire: &Value) -> Vec<(String, String)> {
    let mut found = Vec::new();
    walk(wire, &mut |node| {
        if node.get("hasOnClick") == Some(&Value::Bool(true)) {
            let id = node
                .get("id")
                .and_then(Value::as_str)
                .expect("a widget with a handler was given an id")
                .to_string();
            let kind = node
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            found.push((id, kind));
        }
    });
    found
}

fn walk(value: &Value, visit: &mut impl FnMut(&Value)) {
    match value {
        Value::Object(map) => {
            visit(value);
            for child in map.values() {
                walk(child, visit);
            }
        }
        Value::Array(items) => {
            for item in items {
                walk(item, visit);
            }
        }
        _ => {}
    }
}
