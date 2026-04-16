use rusty::prelude::*;

struct CounterApp;

impl View for CounterApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let count = use_state(ctx, 0i32);

        let count_display = count.clone();
        let count_inc = count.clone();
        let count_dec = count.clone();

        Layout::vertical()
            .gap(16.0)
            .padding(24.0)
            .child(TextBlock::h1("Counter Example"))
            .child(TextBlock::new(&format!("Count: {}", count_display.get())))
            .child(
                Layout::horizontal()
                    .gap(8.0)
                    .child(
                        Button::new("Increment").on_click(move || {
                            count_inc.update(|v| v + 1);
                        }),
                    )
                    .child(
                        Button::new("Decrement").on_click(move || {
                            count_dec.update(|v| v - 1);
                        }),
                    )
                    .child(
                        Button::new("Reset")
                            .variant(button::ButtonVariant::Ghost)
                            .on_click(move || {
                                count.set(0);
                            }),
                    ),
            )
            .into()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    RustyServer::new(3000, || CounterApp).serve().await
}
