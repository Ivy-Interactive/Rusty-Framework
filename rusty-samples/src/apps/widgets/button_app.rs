use rusty::prelude::*;
use rusty::widgets::button::ButtonVariant;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "button",
        title: "Button",
        icon: "mouse-pointer-click",
        group: "Widgets",
        order: 0,
        factory: build,
    }
}

fn build(ctx: &mut BuildContext) -> Element {
    let click_log = use_state(ctx, "No button clicked yet".to_string());

    let log = click_log.clone();

    sample_page(
        "Button",
        "Showcases all Button variants, icons, colors, and states.",
        Layout::vertical()
            .gap(16.0)
            .child(demo_section(
                "Variants",
                Layout::horizontal()
                    .gap(8.0)
                    .child(Button::new("Primary").variant(ButtonVariant::Primary))
                    .child(Button::new("Secondary").variant(ButtonVariant::Secondary))
                    .child(Button::new("Outline").variant(ButtonVariant::Outline))
                    .child(Button::new("Ghost").variant(ButtonVariant::Ghost))
                    .child(Button::new("Danger").variant(ButtonVariant::Danger))
                    .into(),
            ))
            .child(demo_section(
                "With Icons",
                Layout::horizontal()
                    .gap(8.0)
                    .child(Button::new("Save").icon("save"))
                    .child(
                        Button::new("Delete")
                            .icon("trash")
                            .variant(ButtonVariant::Danger),
                    )
                    .child(
                        Button::new("Settings")
                            .icon("settings")
                            .variant(ButtonVariant::Outline),
                    )
                    .into(),
            ))
            .child(demo_section(
                "Colors",
                Layout::horizontal()
                    .gap(8.0)
                    .child(Button::new("Success").color(NamedColor::Success.into()))
                    .child(Button::new("Warning").color(NamedColor::Warning.into()))
                    .child(Button::new("Danger").color(NamedColor::Danger.into()))
                    .child(Button::new("Info").color(NamedColor::Info.into()))
                    .into(),
            ))
            .child(demo_section(
                "States",
                Layout::horizontal()
                    .gap(8.0)
                    .child(Button::new("Disabled").disabled(true))
                    .child(Button::new("Loading").loading(true))
                    .into(),
            ))
            .child(demo_section(
                "Density",
                Layout::horizontal()
                    .gap(8.0)
                    .align(Align::Center)
                    .child(Button::new("Compact").density(Density::Compact))
                    .child(Button::new("Normal").density(Density::Normal))
                    .child(Button::new("Comfortable").density(Density::Comfortable))
                    .into(),
            ))
            .child(demo_section(
                "Click Events",
                Layout::vertical()
                    .gap(8.0)
                    .child(TextBlock::new(&click_log.get()))
                    .child(Button::new("Click Me").on_click(move || {
                        log.set("Button was clicked!".to_string());
                    }))
                    .into(),
            ))
            .into(),
    )
}
