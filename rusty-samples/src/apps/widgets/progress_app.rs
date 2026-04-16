use rusty::prelude::*;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "progress",
        title: "Progress",
        icon: "loader",
        group: "Widgets",
        order: 6,
        factory: build,
    }
}

fn build(ctx: &mut BuildContext) -> Element {
    let value = use_state(ctx, 65.0f64);

    let val = value.get();
    let inc = value.clone();
    let dec = value.clone();

    sample_page(
        "Progress",
        "Demonstrates the Progress bar widget.",
        Layout::vertical()
            .gap(16.0)
            .child(demo_section(
                "Determinate Progress",
                Layout::vertical()
                    .gap(8.0)
                    .child(Progress::new(val).label(&format!("{val:.0}%")))
                    .child(
                        Layout::horizontal()
                            .gap(8.0)
                            .child(Button::new("+10").on_click(move || {
                                inc.update(|v| (v + 10.0).min(100.0));
                            }))
                            .child(Button::new("-10").on_click(move || {
                                dec.update(|v| (v - 10.0).max(0.0));
                            })),
                    )
                    .into(),
            ))
            .child(demo_section(
                "Indeterminate Progress",
                Layout::vertical()
                    .gap(8.0)
                    .child(Progress::indeterminate().label("Loading..."))
                    .child(TextBlock::paragraph(
                        "An indeterminate progress bar shows activity without a specific percentage.",
                    ))
                    .into(),
            ))
            .child(demo_section(
                "Colored Progress",
                Layout::vertical()
                    .gap(8.0)
                    .child(
                        Progress::new(80.0)
                            .color(NamedColor::Success.into())
                            .label("Success"),
                    )
                    .child(
                        Progress::new(45.0)
                            .color(NamedColor::Warning.into())
                            .label("Warning"),
                    )
                    .child(
                        Progress::new(20.0)
                            .color(NamedColor::Danger.into())
                            .label("Danger"),
                    )
                    .into(),
            ))
            .into(),
    )
}
