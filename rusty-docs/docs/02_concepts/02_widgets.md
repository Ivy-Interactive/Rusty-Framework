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

### Widget Type Names on the Wire

Rusty emits widget type names in `snake_case` (e.g., `"data_table"`, `"qr_code"`, `"rich_text_input"`). The type string lives in each widget's `to_json()` implementation, not in `widget_type()`.

The vendored Ivy React frontend at `src/frontend` expects widget type names in the format `"Ivy.PascalCase"` (e.g., `"Ivy.DataTable"`, `"Ivy.Terminal"`). The `shared::widget_names` module records the mapping between Rusty's `snake_case` names and Ivy's keys:

- 14 widgets map mechanically (`badge` → `"Ivy.Badge"`, `button` → `"Ivy.Button"`, etc.)
- 2 widgets are renamed (`select` → `"Ivy.SelectInput"`, `checkbox` → `"Ivy.BoolInput"`)
- 1 widget maps one-to-many (`layout` → `"Ivy.StackLayout"` or `"Ivy.GridLayout"` depending on the `direction` prop)
- 4 widgets have no Ivy counterpart (`activity_heatmap`, `diff_view`, `qr_code`, `rich_text_input`)

```rust
use rusty::shared::{ivy_widget, ivy_widget_for, IvyWidget};

// Look up the mapping
match ivy_widget("button") {
    Some(IvyWidget::One(key)) => println!("Maps to {}", key), // "Ivy.Button"
    _ => {}
}

// Resolve the concrete key from a serialized widget
let button_json = Button::new("Click").to_json();
let ivy_key = ivy_widget_for(&button_json); // Some("Ivy.Button")
```

**Note:** Wiring `src/frontend` to Rusty requires more than translating type names. An adapter must also:

1. Nest props under a `props` object (Ivy's `WidgetNode` structure)
2. Build an `events: string[]` array from Rusty's `has<Event>` booleans
3. Reconcile three event casings:
   - Rust event registration uses lowercase (`"click"`)
   - The E2E harness sends camelCase (`"onClick"`)
   - Ivy widgets expect PascalCase (`"OnClick"`)
