use rusty::prelude::*;
use rusty::widgets::button::ButtonVariant;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "card",
        title: "Card",
        icon: "square",
        group: "Widgets",
        order: 1,
        factory: build,
    }
}

fn build(_ctx: &mut BuildContext) -> Element {
    sample_page(
        "Card",
        "Demonstrates the Card widget with title, subtitle, children, and footer.",
        Layout::vertical()
            .gap(16.0)
            .child(demo_section(
                "Basic Card",
                Card::new()
                    .title("Simple Card")
                    .child(TextBlock::paragraph(
                        "This is a basic card with a title and content.",
                    ))
                    .into(),
            ))
            .child(demo_section(
                "Card with Subtitle",
                Card::new()
                    .title("Project Update")
                    .subtitle("Last updated: today")
                    .child(TextBlock::paragraph(
                        "Everything is on track for the next release.",
                    ))
                    .into(),
            ))
            .child(demo_section(
                "Card with Footer",
                Card::new()
                    .title("Confirmation")
                    .child(TextBlock::paragraph("Are you sure you want to proceed?"))
                    .footer(vec![
                        Button::new("Cancel").variant(ButtonVariant::Ghost).into(),
                        Button::new("Confirm").into(),
                    ])
                    .into(),
            ))
            .child(demo_section(
                "Card with Padding",
                Card::new()
                    .title("Padded Card")
                    .padding(32.0)
                    .child(TextBlock::paragraph("This card has extra padding (32px)."))
                    .into(),
            ))
            .into(),
    )
}
