# Rusty-Framework

Build full-stack web applications in pure Rust.

A direct Rust port of [Ivy-Framework](https://github.com/Ivy-Interactive/Ivy-Framework).

## Quick Start

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
rusty = { git = "https://github.com/Ivy-Interactive/Rusty-Framework" }
```

Build a reactive application:

```rust
use rusty::prelude::*;

struct CounterApp;

impl View for CounterApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let count = use_state(ctx, 0);

        let count_display = count.clone();
        let count_inc = count.clone();

        Layout::vertical()
            .child(TextBlock::new(&format!("Count: {}", count_display.get())))
            .child(Button::new("Increment").on_click(move || count_inc.update(|v| v + 1)))
            .into()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    RustyServer::new(3000, || CounterApp).serve().await
}
```

## Architecture

Rusty-Framework follows the same architecture as Ivy-Framework:

- **Views** — Stateful components implementing the `View` trait with a `build()` method
- **Widgets** — Serializable UI primitives (Button, Text, Layout, Card, etc.) sent as JSON to the frontend
- **Hooks** — React-style state management (`use_state`, `use_effect`, `use_memo`, `use_callback`)
- **Server** — WebSocket server (via axum) that communicates with the React frontend using JSON patches
- **Reconciler** — Diffs widget trees and sends minimal incremental updates

## Crate Structure

| Crate | Description |
|-------|-------------|
| `rusty` | Core framework — views, hooks, widgets, server, shared types |
| `rusty-macros` | Proc macros for `#[derive(Widget)]`, `#[prop]`, `#[event]` |
| `rusty-server` | Standalone server binary |

## Examples

```bash
# Run the counter example
cargo run --example counter

# Run the hello world example
cargo run --example hello_world
```

## Development

```bash
# Build everything
cargo build --workspace

# Run all tests
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all
```

## License

MIT
