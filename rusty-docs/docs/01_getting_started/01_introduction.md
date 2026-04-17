## What is Rusty-Framework?

Rusty-Framework is a Rust-native reactive UI framework inspired by [Ivy-Framework](https://github.com/Ivy-Interactive/Ivy-Framework). It lets you build interactive web applications entirely in Rust using a component-based architecture with server-side rendering over WebSocket.

### Key Features

- **Pure Rust** — write your entire UI in Rust, no JavaScript required
- **Reactive** — fine-grained reactivity via hooks (`use_state`, `use_effect`, `use_memo`)
- **Server-side** — your views run on the server; the browser renders a lightweight widget tree
- **Diff-based updates** — only changed widgets are sent to the client
- **Type-safe** — leverage Rust's type system for widget props and events

### How It Works

1. You define a **View** — a struct that implements the `View` trait
2. The `build()` method returns an `Element` tree made of widgets
3. `RustyServer` serves your view over WebSocket
4. The client renders the widget tree and sends events back
5. Events trigger state changes, which trigger rebuilds, which produce diffs

### Comparison with Ivy-Framework

| Feature | Ivy (C#) | Rusty (Rust) |
|---------|----------|--------------|
| Language | C# | Rust |
| Runtime | .NET | Tokio |
| Transport | WebSocket | WebSocket |
| State | `UseState<T>` | `use_state(ctx, T)` |
| Components | `IView` | `View` trait |
| Widgets | Class-based | Builder pattern |
