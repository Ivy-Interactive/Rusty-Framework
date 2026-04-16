use rusty::prelude::*;

use crate::apps::AppEntry;
use crate::sample_base::sample_page;

pub fn entry() -> AppEntry {
    AppEntry {
        id: "hello",
        title: "Hello World",
        icon: "hand",
        group: "Demos",
        order: 0,
        factory: build,
    }
}

fn build(_ctx: &mut BuildContext) -> Element {
    sample_page(
        "Hello World",
        "A minimal example showing text and layout basics.",
        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h2("Welcome to Rusty Framework!"))
            .child(TextBlock::paragraph(
                "This is a simple hello world example demonstrating TextBlock and Layout widgets.",
            ))
            .child(Card::new().title("About").child(TextBlock::paragraph(
                "Rusty Framework is a server-driven UI framework for Rust, \
                         inspired by Ivy Framework for .NET.",
            )))
            .into(),
    )
}
