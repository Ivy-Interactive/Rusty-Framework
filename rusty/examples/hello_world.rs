use rusty::prelude::*;

struct HelloApp;

impl View for HelloApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .gap(16.0)
            .padding(24.0)
            .child(TextBlock::h1("Hello, World!"))
            .child(TextBlock::paragraph(
                "This is a Rusty-Framework application.",
            ))
            .child(Button::new("Click me").on_click(|| {
                println!("Button clicked!");
            }))
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

    RustyServer::new(port, || HelloApp).serve().await
}
