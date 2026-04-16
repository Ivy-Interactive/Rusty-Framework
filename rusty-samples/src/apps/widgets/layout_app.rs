use rusty::prelude::*;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "layout",
        title: "Layout",
        icon: "layout-grid",
        group: "Widgets",
        order: 8,
        factory: build,
    }
}

fn build(_ctx: &mut BuildContext) -> Element {
    let box_item =
        |label: &str| -> Element { Badge::new(label).color(NamedColor::Primary.into()).into() };

    sample_page(
        "Layout",
        "Demonstrates vertical, horizontal, and grid layouts with gap, align, and justify.",
        Layout::vertical()
            .gap(16.0)
            .child(demo_section(
                "Vertical Layout",
                Layout::vertical()
                    .gap(8.0)
                    .child(box_item("Item 1"))
                    .child(box_item("Item 2"))
                    .child(box_item("Item 3"))
                    .into(),
            ))
            .child(demo_section(
                "Horizontal Layout",
                Layout::horizontal()
                    .gap(8.0)
                    .child(box_item("Item 1"))
                    .child(box_item("Item 2"))
                    .child(box_item("Item 3"))
                    .into(),
            ))
            .child(demo_section(
                "Grid Layout (3 columns)",
                Layout::grid(3)
                    .gap(8.0)
                    .child(box_item("1"))
                    .child(box_item("2"))
                    .child(box_item("3"))
                    .child(box_item("4"))
                    .child(box_item("5"))
                    .child(box_item("6"))
                    .into(),
            ))
            .child(demo_section(
                "Gap Variations",
                Layout::vertical()
                    .gap(12.0)
                    .child(TextBlock::new("Gap: 4px"))
                    .child(
                        Layout::horizontal()
                            .gap(4.0)
                            .child(box_item("A"))
                            .child(box_item("B"))
                            .child(box_item("C")),
                    )
                    .child(TextBlock::new("Gap: 16px"))
                    .child(
                        Layout::horizontal()
                            .gap(16.0)
                            .child(box_item("A"))
                            .child(box_item("B"))
                            .child(box_item("C")),
                    )
                    .child(TextBlock::new("Gap: 32px"))
                    .child(
                        Layout::horizontal()
                            .gap(32.0)
                            .child(box_item("A"))
                            .child(box_item("B"))
                            .child(box_item("C")),
                    )
                    .into(),
            ))
            .child(demo_section(
                "Alignment",
                Layout::vertical()
                    .gap(12.0)
                    .child(TextBlock::new("Align: Start"))
                    .child(
                        Layout::horizontal()
                            .gap(8.0)
                            .align(Align::Start)
                            .child(box_item("Start")),
                    )
                    .child(TextBlock::new("Align: Center"))
                    .child(
                        Layout::horizontal()
                            .gap(8.0)
                            .align(Align::Center)
                            .child(box_item("Center")),
                    )
                    .child(TextBlock::new("Align: End"))
                    .child(
                        Layout::horizontal()
                            .gap(8.0)
                            .align(Align::End)
                            .child(box_item("End")),
                    )
                    .into(),
            ))
            .child(demo_section(
                "Justify Content",
                Layout::vertical()
                    .gap(12.0)
                    .child(TextBlock::new("Justify: SpaceBetween"))
                    .child(
                        Layout::horizontal()
                            .gap(8.0)
                            .justify(Justify::SpaceBetween)
                            .child(box_item("Left"))
                            .child(box_item("Right")),
                    )
                    .child(TextBlock::new("Justify: SpaceAround"))
                    .child(
                        Layout::horizontal()
                            .gap(8.0)
                            .justify(Justify::SpaceAround)
                            .child(box_item("A"))
                            .child(box_item("B"))
                            .child(box_item("C")),
                    )
                    .into(),
            ))
            .into(),
    )
}
