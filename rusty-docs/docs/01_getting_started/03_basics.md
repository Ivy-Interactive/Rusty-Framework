## Basics

### The View Trait

Every UI component in Rusty-Framework implements the `View` trait:

```rust
pub trait View: Send + Sync + 'static {
    fn build(&self, ctx: &mut BuildContext) -> Element;
}
```

The `build()` method receives a `BuildContext` and returns an `Element` — the tree of widgets to render.

### Your First View

```rust
use rusty::prelude::*;

struct HelloApp;

impl View for HelloApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .gap(16.0)
            .padding(24.0)
            .child(TextBlock::h1("Hello, World!"))
            .child(TextBlock::paragraph("Welcome to Rusty-Framework."))
            .into()
    }
}
```

### Running the Server

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    RustyServer::new(3000, || HelloApp).serve().await
}
```

This starts a WebSocket server on port 3000. Open your browser and connect to see the rendered UI.

### The Element Tree

`Element` is an enum with three variants:

- `Element::Widget(Box<dyn WidgetData>)` — a concrete widget
- `Element::Fragment(Vec<Element>)` — a list of elements
- `Element::Empty` — renders nothing

Widgets convert to `Element` via the `.into()` method, which is called at the end of a builder chain.
