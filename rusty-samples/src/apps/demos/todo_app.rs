use rusty::prelude::*;
use rusty::widgets::button::ButtonVariant;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "todo",
        title: "Todo List",
        icon: "list-checks",
        group: "Demos",
        order: 2,
        factory: build,
    }
}

fn build(ctx: &mut BuildContext) -> Element {
    let items = use_state(
        ctx,
        vec!["Buy groceries".to_string(), "Write code".to_string()],
    );
    let input_value = use_state(ctx, String::new());

    let input_display = input_value.get();

    let input_change = input_value.clone();
    let add_items = items.clone();
    let add_input = input_value.clone();

    let current_items = items.get();

    let mut item_elements: Vec<Element> = Vec::new();
    for (i, item) in current_items.iter().enumerate() {
        let remove_items = items.clone();
        item_elements.push(
            Layout::horizontal()
                .gap(8.0)
                .align(Align::Center)
                .child(TextBlock::new(item))
                .child(
                    Button::new("Remove")
                        .variant(ButtonVariant::Danger)
                        .density(Density::Compact)
                        .on_click(move || {
                            remove_items.update(|list| {
                                let mut new_list = list.clone();
                                if i < new_list.len() {
                                    new_list.remove(i);
                                }
                                new_list
                            });
                        }),
                )
                .into(),
        );
    }

    sample_page(
        "Todo List",
        "Demonstrates use_state with collections, TextInput, and dynamic lists.",
        demo_section(
            "Tasks",
            Layout::vertical()
                .gap(12.0)
                .child(
                    Layout::horizontal()
                        .gap(8.0)
                        .align(Align::Center)
                        .child(
                            TextInput::new()
                                .value(&input_display)
                                .placeholder("Add a task...")
                                .on_change(move |val: String| {
                                    input_change.set(val);
                                }),
                        )
                        .child(Button::new("Add").on_click(move || {
                            let val = add_input.get();
                            if !val.is_empty() {
                                add_items.update(|list| {
                                    let mut new_list = list.clone();
                                    new_list.push(val.clone());
                                    new_list
                                });
                                add_input.set(String::new());
                            }
                        })),
                )
                .child(Layout::vertical().gap(4.0).children(item_elements))
                .child(TextBlock::paragraph(&format!(
                    "{} task(s) remaining",
                    current_items.len()
                )))
                .into(),
        ),
    )
}
