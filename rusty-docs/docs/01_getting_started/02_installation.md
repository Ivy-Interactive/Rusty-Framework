## Installation

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- A modern web browser

### Adding to an Existing Workspace

Add `rusty` as a dependency in your crate's `Cargo.toml`:

```toml
[dependencies]
rusty = { path = "../rusty" }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

### Creating a New Project

```bash
cargo new my-app
cd my-app
```

Then add the dependencies above and you're ready to build your first view.
