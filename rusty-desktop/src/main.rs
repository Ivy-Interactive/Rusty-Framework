use clap::Parser;
use rusty::prelude::*;
use rusty_desktop::desktop_router;
use rusty_desktop::menu::default_menu;
use rusty_desktop::shell::{self, ShellOptions, DEFAULT_TITLE};

#[derive(Parser)]
#[command(name = "rusty-desktop")]
#[command(about = "Run a Rusty-Framework application as a native desktop app")]
struct Cli {
    /// Native window title
    #[arg(long, default_value = DEFAULT_TITLE, env = "RUSTY_TITLE")]
    title: String,

    /// Open the WebView inspector. Requires a debug build or --features devtools.
    #[arg(long)]
    devtools: bool,
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

/// The menu's View submenu is built from this registry. The window itself always shows the
/// server's root view; navigation is what routes to a registered app.
fn registry() -> AppRegistry {
    let mut registry = AppRegistry::new();
    registry.register("home", "Home", std::sync::Arc::new(|| Box::new(HelloWorld)));
    registry
}

// Not `#[tokio::main]`: `tao` must build its event loop on the main thread, and the shell
// owns its own runtime so the server outlives the call that starts it.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    // Port 0: the shell binds an ephemeral loopback port and points the WebView at it.
    // No `RUSTY_PORT=` line — that exists for the E2E harness scanner, which has no
    // counterpart here.
    let server = RustyServer::new(0, || HelloWorld);
    let spec = default_menu(&registry());

    shell::run(
        spec,
        desktop_router(server),
        ShellOptions {
            title: cli.title,
            devtools: cli.devtools || cfg!(debug_assertions),
        },
    )
}
