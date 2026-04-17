use rusty::prelude::*;

mod generated;
mod server;

use server::DocsShellView;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    RustyServer::new(3001, || DocsShellView).serve().await
}
