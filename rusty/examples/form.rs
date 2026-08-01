//! A form exercising the event round-trip for all four input widgets.
//!
//! Run with `cargo run --example form` (set `PORT` to override 3000).

use rusty::prelude::*;
use rusty::widgets::input::SelectOption;

struct FormApp;

impl View for FormApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let name = use_state(ctx, String::new());
        let quantity = use_state(ctx, 1.0f64);
        let fruit = use_state(ctx, "apple".to_string());
        let subscribe = use_state(ctx, false);
        let submitted = use_state(ctx, false);

        let name_val = name.get();
        let quantity_val = quantity.get();
        let fruit_val = fruit.get();
        let subscribe_val = subscribe.get();

        let name_set = name.clone();
        let quantity_set = quantity.clone();
        let fruit_set = fruit.clone();
        let subscribe_set = subscribe.clone();
        let submitted_set = submitted.clone();

        let mut form = Layout::vertical()
            .padding(24.0)
            .gap(16.0)
            .child(TextBlock::h1("Form Example"))
            .child(
                TextInput::new()
                    .label("Name")
                    .placeholder("Enter your name")
                    .value(&name_val)
                    .on_change(move |v: String| name_set.set(v)),
            )
            .child(
                NumberInput::new()
                    .label("Quantity")
                    .value(quantity_val)
                    .min(1.0)
                    .max(99.0)
                    .step(1.0)
                    .on_change(move |v: f64| quantity_set.set(v)),
            )
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
                    SelectOption {
                        value: "cherry".into(),
                        label: "Cherry".into(),
                    },
                ])
                .label("Fruit")
                .value(&fruit_val)
                .on_change(move |v: String| fruit_set.set(v)),
            )
            .child(
                Checkbox::new(subscribe_val)
                    .label("Subscribe to updates")
                    .on_change(move |v: bool| subscribe_set.set(v)),
            )
            .child(Button::new("Submit").on_click(move || submitted_set.set(true)));

        if submitted.get() {
            form = form.child(
                Card::new()
                    .title("Submitted")
                    .child(TextBlock::paragraph(&format!("Name: {}", name_val)))
                    .child(TextBlock::paragraph(&format!("Quantity: {}", quantity_val)))
                    .child(TextBlock::paragraph(&format!("Fruit: {}", fruit_val)))
                    .child(TextBlock::paragraph(&format!(
                        "Subscribed: {}",
                        subscribe_val
                    ))),
            );
        }

        form.into()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    RustyServer::new(port, || FormApp).serve().await
}
