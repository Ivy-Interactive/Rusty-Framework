use rusty::prelude::*;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "tooltip",
        title: "Tooltip",
        icon: "message-circle",
        group: "Widgets",
        order: 7,
        factory: build,
    }
}

fn build(_ctx: &mut BuildContext) -> Element {
    sample_page(
        "Tooltip",
        "Demonstrates the Tooltip widget wrapping various elements.",
        Layout::vertical()
            .gap(16.0)
            .child(demo_section(
                "Tooltip on Button",
                Layout::horizontal()
                    .gap(8.0)
                    .child(Tooltip::new(
                        "Click to save your work",
                        Button::new("Save").icon("save"),
                    ))
                    .child(Tooltip::new(
                        "This will delete the item permanently",
                        Button::new("Delete").icon("trash"),
                    ))
                    .into(),
            ))
            .child(demo_section(
                "Tooltip on Text",
                Layout::vertical()
                    .gap(8.0)
                    .child(Tooltip::new(
                        "This is additional context for the text",
                        TextBlock::new("Hover over me for more info"),
                    ))
                    .into(),
            ))
            .child(demo_section(
                "Tooltip on Badge",
                Layout::horizontal()
                    .gap(8.0)
                    .child(Tooltip::new(
                        "All systems operational",
                        Badge::new("Active").color(NamedColor::Success.into()),
                    ))
                    .child(Tooltip::new(
                        "Requires attention",
                        Badge::new("Warning").color(NamedColor::Warning.into()),
                    ))
                    .into(),
            ))
            .into(),
    )
}
