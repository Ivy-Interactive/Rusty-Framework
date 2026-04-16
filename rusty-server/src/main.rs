use rusty::prelude::*;

struct HelloWorld;

impl View for HelloWorld {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Welcome to Rusty-Framework"))
            .child(TextBlock::paragraph(
                "Build full-stack web applications in pure Rust.",
            ))
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

    tracing::info!("Starting Rusty-Framework server on port {}", port);
    RustyServer::new(port, || HelloWorld).serve().await
}
