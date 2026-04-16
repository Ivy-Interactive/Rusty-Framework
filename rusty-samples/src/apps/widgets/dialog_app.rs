use rusty::prelude::*;
use rusty::widgets::button::ButtonVariant;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "dialog",
        title: "Dialog",
        icon: "panel-top",
        group: "Widgets",
        order: 4,
        factory: build,
    }
}

fn build(ctx: &mut BuildContext) -> Element {
    let open = use_state(ctx, false);

    let is_open = open.get();
    let toggle = open.clone();
    let close = open.clone();
    let close2 = open.clone();

    sample_page(
        "Dialog",
        "Demonstrates the Dialog widget with open/close state, content, and footer.",
        Layout::vertical()
            .gap(16.0)
            .child(demo_section(
                "Interactive Dialog",
                Layout::vertical()
                    .gap(8.0)
                    .child(Button::new("Open Dialog").on_click(move || {
                        toggle.set(true);
                    }))
                    .child(TextBlock::new(&format!(
                        "Dialog is {}",
                        if is_open { "open" } else { "closed" }
                    )))
                    .child(
                        Dialog::new(is_open)
                            .title("Confirm Action")
                            .child(TextBlock::paragraph(
                                "Are you sure you want to perform this action? This cannot be undone.",
                            ))
                            .footer(vec![
                                Button::new("Cancel")
                                    .variant(ButtonVariant::Ghost)
                                    .on_click(move || {
                                        close.set(false);
                                    })
                                    .into(),
                                Button::new("Confirm")
                                    .on_click(move || {
                                        close2.set(false);
                                    })
                                    .into(),
                            ]),
                    )
                    .into(),
            ))
            .into(),
    )
}
