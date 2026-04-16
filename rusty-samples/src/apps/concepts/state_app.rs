use rusty::prelude::*;
use rusty::widgets::button::ButtonVariant;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "state",
        title: "State",
        icon: "database",
        group: "Concepts",
        order: 0,
        factory: build,
    }
}

fn build(ctx: &mut BuildContext) -> Element {
    // Primitive state
    let counter = use_state(ctx, 0i32);
    let text = use_state(ctx, "Hello".to_string());

    // Struct-like state (using a tuple)
    let point = use_state(ctx, (0i32, 0i32));

    // Collection state
    let tags = use_state(ctx, vec!["rust".to_string(), "framework".to_string()]);

    let counter_val = counter.get();
    let text_val = text.get();
    let point_val = point.get();
    let tags_val = tags.get();

    let inc = counter.clone();
    let text_set = text.clone();
    let point_set = point.clone();
    let tags_add = tags.clone();
    let tags_clear = tags.clone();

    sample_page(
        "State",
        "Demonstrates use_state patterns with primitives, structs, and collections.",
        Layout::vertical()
            .gap(16.0)
            .child(demo_section(
                "Primitive State",
                Layout::vertical()
                    .gap(8.0)
                    .child(TextBlock::new(&format!("Counter: {counter_val}")))
                    .child(Button::new("Increment").on_click(move || {
                        inc.update(|v| v + 1);
                    }))
                    .into(),
            ))
            .child(demo_section(
                "String State",
                Layout::vertical()
                    .gap(8.0)
                    .child(TextBlock::new(&format!("Text: {text_val}")))
                    .child(
                        TextInput::new()
                            .value(&text_val)
                            .placeholder("Type something...")
                            .on_change(move |val: String| {
                                text_set.set(val);
                            }),
                    )
                    .into(),
            ))
            .child(demo_section(
                "Tuple State (Point)",
                Layout::vertical()
                    .gap(8.0)
                    .child(TextBlock::new(&format!(
                        "Point: ({}, {})",
                        point_val.0, point_val.1
                    )))
                    .child(
                        Layout::horizontal()
                            .gap(8.0)
                            .child(Button::new("Move Right").on_click(move || {
                                point_set.update(|(x, y)| (x + 1, *y));
                            })),
                    )
                    .into(),
            ))
            .child(demo_section(
                "Collection State",
                Layout::vertical()
                    .gap(8.0)
                    .child(TextBlock::new(&format!("Tags: {}", tags_val.join(", "))))
                    .child(
                        Layout::horizontal()
                            .gap(8.0)
                            .child(Button::new("Add Tag").on_click(move || {
                                tags_add.update(|t| {
                                    let mut new = t.clone();
                                    new.push(format!("tag-{}", new.len()));
                                    new
                                });
                            }))
                            .child(Button::new("Clear").variant(ButtonVariant::Ghost).on_click(
                                move || {
                                    tags_clear.set(Vec::new());
                                },
                            )),
                    )
                    .into(),
            ))
            .into(),
    )
}
