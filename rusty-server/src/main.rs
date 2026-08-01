use clap::Parser;
use rusty::prelude::*;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rusty-server")]
#[command(about = "Serve a Rusty-Framework application")]
struct Cli {
    /// Port to listen on
    #[arg(short, long, env = "PORT", default_value = "3000")]
    port: u16,

    /// Address to bind to. Defaults to loopback; pass 0.0.0.0 to expose on the network.
    #[arg(long, default_value = DEFAULT_BIND_ADDRESS, env = "HOST")]
    host: String,

    /// Directory to serve static files from
    #[arg(short, long)]
    static_dir: Option<PathBuf>,
}

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
    let cli = Cli::parse();

    tracing::info!("Starting Rusty-Framework server on port {}", cli.port);
    let server = RustyServer::new(cli.port, || HelloWorld);

    let server = if let Some(dir) = cli.static_dir {
        server.with_static_dir(dir)
    } else {
        server
    };

    server.with_bind_address(cli.host).serve().await
}
