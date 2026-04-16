use rusty::prelude::*;
use rusty::widgets::input::SelectOption;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "input",
        title: "Input",
        icon: "text-cursor-input",
        group: "Widgets",
        order: 3,
        factory: build,
    }
}

fn build(ctx: &mut BuildContext) -> Element {
    let text_val = use_state(ctx, "Hello".to_string());
    let number_val = use_state(ctx, 42.0f64);
    let select_val = use_state(ctx, "rust".to_string());
    let check_val = use_state(ctx, false);

    let tv = text_val.get();
    let nv = number_val.get();
    let sv = select_val.get();
    let cv = check_val.get();

    let text_change = text_val.clone();
    let num_change = number_val.clone();
    let sel_change = select_val.clone();
    let chk_change = check_val.clone();

    sample_page(
        "Input",
        "Demonstrates TextInput, NumberInput, Select, and Checkbox widgets.",
        Layout::vertical()
            .gap(16.0)
            .child(demo_section(
                "TextInput",
                Layout::vertical()
                    .gap(8.0)
                    .child(
                        TextInput::new()
                            .value(&tv)
                            .label("Name")
                            .placeholder("Enter your name...")
                            .on_change(move |val: String| {
                                text_change.set(val);
                            }),
                    )
                    .child(TextBlock::new(&format!("Value: {tv}")))
                    .into(),
            ))
            .child(demo_section(
                "NumberInput",
                Layout::vertical()
                    .gap(8.0)
                    .child(
                        NumberInput::new()
                            .value(nv)
                            .min(0.0)
                            .max(100.0)
                            .step(1.0)
                            .label("Amount")
                            .on_change(move |val: f64| {
                                num_change.set(val);
                            }),
                    )
                    .child(TextBlock::new(&format!("Value: {nv}")))
                    .into(),
            ))
            .child(demo_section(
                "Select",
                Layout::vertical()
                    .gap(8.0)
                    .child(
                        Select::new(vec![
                            SelectOption {
                                value: "rust".into(),
                                label: "Rust".into(),
                            },
                            SelectOption {
                                value: "typescript".into(),
                                label: "TypeScript".into(),
                            },
                            SelectOption {
                                value: "python".into(),
                                label: "Python".into(),
                            },
                            SelectOption {
                                value: "go".into(),
                                label: "Go".into(),
                            },
                        ])
                        .value(&sv)
                        .label("Language")
                        .placeholder("Choose a language...")
                        .on_change(move |val: String| {
                            sel_change.set(val);
                        }),
                    )
                    .child(TextBlock::new(&format!("Selected: {sv}")))
                    .into(),
            ))
            .child(demo_section(
                "Checkbox",
                Layout::vertical()
                    .gap(8.0)
                    .child(Checkbox::new(cv).label("I agree to the terms").on_change(
                        move |val: bool| {
                            chk_change.set(val);
                        },
                    ))
                    .child(TextBlock::new(&format!(
                        "Checked: {}",
                        if cv { "Yes" } else { "No" }
                    )))
                    .into(),
            ))
            .into(),
    )
}
