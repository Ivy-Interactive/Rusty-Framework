use clap::Parser;
use rusty::prelude::*;

mod generated;
mod server;

use server::DocsShellView;

#[derive(Parser)]
#[command(name = "rusty-docs")]
#[command(about = "Serve the Rusty-Framework documentation site")]
struct Cli {
    /// Port to listen on
    #[arg(short, long, env = "PORT", default_value = "3001")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    RustyServer::new(cli.port, || DocsShellView).serve().await
}
