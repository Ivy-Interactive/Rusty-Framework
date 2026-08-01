//! One card per widget family, laid out in a two-column grid.
//!
//! Run with `cargo run --example widget_gallery` (set `PORT` to override 3000).

use rusty::prelude::*;
use rusty::widgets::badge::BadgeVariant;
use rusty::widgets::button::ButtonVariant;
use rusty::widgets::input::SelectOption;
use rusty::widgets::table::Column;

struct GalleryApp;

impl View for GalleryApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .padding(24.0)
            .gap(16.0)
            .child(TextBlock::h1("Widget Gallery"))
            .child(
                Layout::grid(2)
                    .gap(16.0)
                    .child(
                        Card::new()
                            .title("Text")
                            .child(TextBlock::h2("Heading 2"))
                            .child(TextBlock::paragraph("A paragraph of body text."))
                            .child(TextBlock::code("let x = 42;"))
                            .child(TextBlock::label("A label")),
                    )
                    .child(
                        Card::new()
                            .title("Buttons")
                            .child(Button::new("Primary").variant(ButtonVariant::Primary))
                            .child(Button::new("Secondary").variant(ButtonVariant::Secondary))
                            .child(Button::new("Outline").variant(ButtonVariant::Outline))
                            .child(Button::new("Ghost").variant(ButtonVariant::Ghost))
                            .child(Button::new("Danger").variant(ButtonVariant::Danger)),
                    )
                    .child(
                        Card::new()
                            .title("Inputs")
                            .child(TextInput::new().label("Name").placeholder("Enter a name"))
                            .child(NumberInput::new().label("Amount").value(42.0))
                            .child(
                                Select::new(vec![
                                    SelectOption {
                                        value: "apple".into(),
                                        label: "Apple".into(),
                                    },
                                    SelectOption {
                                        value: "banana".into(),
                                        label: "Banana".into(),
                                    },
                                ])
                                .label("Fruit")
                                .value("apple"),
                            )
                            .child(Checkbox::new(true).label("Enabled")),
                    )
                    .child(
                        Card::new()
                            .title("Badges")
                            .child(Badge::new("Default").variant(BadgeVariant::Default))
                            .child(Badge::new("Outline").variant(BadgeVariant::Outline))
                            .child(Badge::new("Success").color(Color::Named(NamedColor::Success)))
                            .child(Badge::new("Danger").color(Color::Named(NamedColor::Danger))),
                    )
                    .child(
                        Card::new()
                            .title("Progress")
                            .child(Progress::new(0.25).label("Quarter"))
                            .child(Progress::new(0.75).label("Three quarters"))
                            .child(Progress::indeterminate()),
                    )
                    .child(
                        Card::new().title("Table").child(
                            Table::new(vec![
                                Column {
                                    key: "name".into(),
                                    label: "Name".into(),
                                    sortable: true,
                                },
                                Column {
                                    key: "role".into(),
                                    label: "Role".into(),
                                    sortable: false,
                                },
                            ])
                            .rows(vec![
                                serde_json::json!({ "name": "Ada", "role": "Engineer" }),
                                serde_json::json!({ "name": "Grace", "role": "Admiral" }),
                            ]),
                        ),
                    )
                    .child(Card::new().title("Tooltip").child(Tooltip::new(
                        "Tooltips explain things",
                        Button::new("Hover me"),
                    ))),
            )
            .into()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    RustyServer::new(port, || GalleryApp).serve().await
}
