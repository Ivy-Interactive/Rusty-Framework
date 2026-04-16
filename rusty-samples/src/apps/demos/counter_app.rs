use rusty::prelude::*;
use rusty::widgets::button::ButtonVariant;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "counter",
        title: "Counter",
        icon: "hash",
        group: "Demos",
        order: 1,
        factory: build,
    }
}

fn build(ctx: &mut BuildContext) -> Element {
    let count = use_state(ctx, 0i32);

    let count_display = count.get();
    let inc = count.clone();
    let dec = count.clone();
    let reset = count.clone();

    sample_page(
        "Counter",
        "Demonstrates use_state for reactive state and Button events.",
        demo_section(
            "Interactive Counter",
            Layout::vertical()
                .gap(16.0)
                .child(TextBlock::h2(&format!("Count: {count_display}")))
                .child(
                    Layout::horizontal()
                        .gap(8.0)
                        .child(Button::new("Increment").on_click(move || {
                            inc.update(|v| v + 1);
                        }))
                        .child(Button::new("Decrement").on_click(move || {
                            dec.update(|v| v - 1);
                        }))
                        .child(Button::new("Reset").variant(ButtonVariant::Ghost).on_click(
                            move || {
                                reset.set(0);
                            },
                        )),
                )
                .into(),
        ),
    )
}
