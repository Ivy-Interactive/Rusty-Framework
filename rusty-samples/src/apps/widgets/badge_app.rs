use rusty::prelude::*;
use rusty::widgets::badge::BadgeVariant;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "badge",
        title: "Badge",
        icon: "tag",
        group: "Widgets",
        order: 5,
        factory: build,
    }
}

fn build(_ctx: &mut BuildContext) -> Element {
    sample_page(
        "Badge",
        "Demonstrates Badge variants and colors.",
        Layout::vertical()
            .gap(16.0)
            .child(demo_section(
                "Variants",
                Layout::horizontal()
                    .gap(8.0)
                    .child(Badge::new("Default").variant(BadgeVariant::Default))
                    .child(Badge::new("Outline").variant(BadgeVariant::Outline))
                    .child(Badge::new("Dot").variant(BadgeVariant::Dot))
                    .into(),
            ))
            .child(demo_section(
                "Colors",
                Layout::horizontal()
                    .gap(8.0)
                    .child(Badge::new("Primary").color(NamedColor::Primary.into()))
                    .child(Badge::new("Success").color(NamedColor::Success.into()))
                    .child(Badge::new("Warning").color(NamedColor::Warning.into()))
                    .child(Badge::new("Danger").color(NamedColor::Danger.into()))
                    .child(Badge::new("Info").color(NamedColor::Info.into()))
                    .into(),
            ))
            .child(demo_section(
                "Custom Colors",
                Layout::horizontal()
                    .gap(8.0)
                    .child(Badge::new("Hex Color").color(Color::hex("#8b5cf6")))
                    .child(Badge::new("Another").color(Color::hex("#ec4899")))
                    .into(),
            ))
            .into(),
    )
}
