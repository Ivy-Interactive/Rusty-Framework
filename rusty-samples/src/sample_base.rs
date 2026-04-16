use rusty::prelude::*;
use rusty::widgets::text::TextVariant;

/// Helper that wraps a sample's content with a title and description section.
pub fn sample_page(title: &str, description: &str, content: Element) -> Element {
    Layout::vertical()
        .gap(24.0)
        .padding(32.0)
        .child(
            Layout::vertical()
                .gap(8.0)
                .child(TextBlock::h1(title))
                .child(TextBlock::new(description).variant(TextVariant::Paragraph)),
        )
        .child(content)
        .into()
}

/// Helper that wraps a demo section with a subtitle.
pub fn demo_section(title: &str, content: Element) -> Element {
    Card::new().title(title).padding(16.0).child(content).into()
}
