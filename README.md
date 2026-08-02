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
| `rusty-ivyml` | Proc macros for `ivyml!` / `ivyml_file!` declarative markup |
| `rusty-server` | Standalone server binary |

## Examples

The examples live in `rusty/examples/` and each one starts a server on port 3000.
Set `PORT` to run several at once.

```bash
# Smallest possible app — layout, text and a button
cargo run --example hello_world

# use_state driving increment / decrement / reset
cargo run --example counter

# One card per widget family
cargo run --example widget_gallery

# Every input widget wired to state, with a submit summary
cargo run --example form

# Every hook, each with a visible readout
cargo run --example hooks_showcase

# Run two at once
PORT=3010 cargo run --example counter
```

## Development

```bash
# Build everything
cargo build --workspace

# Run all tests
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Format
cargo fmt --all
```

## License

MIT
