## Widgets

Widgets are the visual primitives of Rusty-Framework. They are serializable data structures that describe what to render on the client.

### The WidgetData Trait

Every widget implements `WidgetData`. Four methods are required:

- `widget_type(&self) -> &str` — the type name sent to the client
- `to_json(&self) -> serde_json::Value` — serialization for the wire protocol
- `clone_box(&self) -> Box<dyn WidgetData>` — dynamic cloning
- `assign_id(&mut self, id: String)` and `get_id(&self) -> Option<&str>` — ID management, driven by the automatic ID-assignment tree walk

The rest have default implementations and only need overriding when they apply:

- `register_events(&self, widget_id: &str, registry: &mut EventRegistry)` — bind this widget's handlers into the event registry. Called during the post-build tree walk; the default registers nothing.
- `children_mut(&mut self) -> Option<&mut Vec<Element>>` — expose children for recursive walking. Container widgets (`Layout`, `Card`, `Dialog`) override it; the default returns `None`.
- `single_child_mut(&mut self) -> Option<&mut Element>` — the same for a single wrapped child. `Tooltip` overrides it.
- `footer_mut(&mut self) -> Option<&mut Vec<Element>>` — the same for footer elements. `Card` and `Dialog` override it.

A container that does not expose its children will not have IDs assigned or events registered for them, so overriding the right accessor is what makes nesting work.

The trait is not in the prelude — import it from `rusty::views::view::WidgetData`.

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
use rusty::Widget;
use std::sync::Arc;

#[derive(Widget, Clone)]
struct MyWidget {
    #[prop]
    label: String,
    #[prop]
    count: i32,
    #[event]
    on_click: Option<Arc<dyn Fn() + Send + Sync>>,
}

// Debug cannot be derived alongside an Arc<dyn Fn> field, so write it by hand
// and skip the handler — the same approach Button takes.
impl std::fmt::Debug for MyWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MyWidget")
            .field("label", &self.label)
            .field("count", &self.count)
            .finish()
    }
}
```

- `#[prop]` marks serializable properties
- `#[event]` marks event handler fields, serialized as a `hasOnClick`-style boolean

The generated `widget_type()` is the struct name in snake_case, so `MyWidget` sends `"my_widget"`.

The macro currently expands to `crate::views::...` paths, which means it only compiles **inside the `rusty` crate**. Implement `WidgetData` by hand for widgets defined in your own crate.
