## Widgets

Widgets are the visual primitives of Rusty-Framework. They are serializable data structures that describe what to render on the client.

### The WidgetData Trait

Every widget implements `WidgetData`, which provides:

- `widget_type() -> &'static str` — the type name sent to the client
- `to_json() -> serde_json::Value` — serialization for the wire protocol
- `clone_box() -> Box<dyn WidgetData>` — dynamic cloning
- `assign_id(id: String)` / `get_id() -> Option<String>` — ID management

### Builder Pattern

All widgets use a builder pattern:

```rust
let button = Button::new("Click me")
    .variant(ButtonVariant::Primary)
    .icon(Icon::from("check"))
    .disabled(false)
    .color(Color::Named(NamedColor::Success));
```

### Converting to Element

Every widget implements `From<Widget> for Element`. Call `.into()` at the end of a builder chain:

```rust
fn build(&self, _ctx: &mut BuildContext) -> Element {
    Button::new("OK").into()
}
```

Container widgets accept `impl Into<Element>` in their `.child()` method, so nested widgets convert automatically.

### Custom Widgets with Derive

Use the `#[derive(Widget)]` macro for custom widgets:

```rust
#[derive(Widget, Clone, Debug)]
struct MyWidget {
    #[prop]
    label: String,
    #[prop]
    count: i32,
    #[event]
    on_click: Option<Arc<dyn Fn() + Send + Sync>>,
}
```

- `#[prop]` marks serializable properties
- `#[event]` marks event handler fields
