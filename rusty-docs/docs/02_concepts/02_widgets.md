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

### Widget Type Names on the Wire

Rusty emits widget type names in `snake_case` (e.g., `"data_table"`, `"qr_code"`, `"rich_text_input"`). The type string lives in each widget's `to_json()` implementation, not in `widget_type()`.

The vendored Ivy React frontend at `src/frontend` expects widget type names in the format `"Ivy.PascalCase"` (e.g., `"Ivy.DataTable"`, `"Ivy.Terminal"`). The `shared::widget_names` module records the mapping between Rusty's `snake_case` names and Ivy's keys:

All 45 widget types have an entry:

- 32 widgets map mechanically (`badge` → `"Ivy.Badge"`, `button` → `"Ivy.Button"`, etc.)
- 4 widgets are renamed (`select` → `"Ivy.SelectInput"`, `checkbox` → `"Ivy.BoolInput"`, `container` → `"Ivy.Box"`, `date_input` → `"Ivy.DateTimeInput"`)
- 1 widget maps one-to-many (`layout` → `"Ivy.StackLayout"` or `"Ivy.GridLayout"` depending on the `direction` prop)
- 1 widget collapses into a *variant* of another (`text_area` → `"Ivy.TextInput"` with `variant: "Textarea"`, since Ivy has no textarea widget). This is `IvyWidget::WithProp`, which synthesizes a prop Rust never sends — as distinct from `ByProp`, which reads one Rust already sends.
- 7 widgets have no Ivy counterpart (`activity_heatmap`, `diff_view`, `multi_select`, `qr_code`, `radio_group`, `rich_text_input`, `slider`)

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

### Translating a Whole Node

Type names are only part of the gap. `shared::ivy_node` translates a serialized Rusty widget tree into Ivy's `WidgetNode` shape — nesting props, deriving the events array, recasing enum values, and normalizing children. It is pure and, like `widget_names`, unused by Rusty itself.

```rust
use rusty::shared::to_ivy_node;

let node = to_ivy_node(&Button::new("Save").on_click(|| {}).to_json()).unwrap();
// { "type": "Ivy.Button", "id": "…", "props": { "title": "Save", … },
//   "children": [], "events": ["OnClick"] }
```

`to_ivy_node` returns `None` for a `RustOnly` or unknown type; such children are dropped from `children` rather than emitted as `null`, since Ivy maps over the array unconditionally.

Three things a reader cannot infer from the mapping table:

- **`terminal`'s `OnResize` and `OnInput` have nowhere to land.** `terminal` maps to `Ivy.Terminal`, but `TerminalWidget.tsx` reads no `events` prop, so those two names — along with `OnToggle` (`expandable`), `OnDayClick` and `OnLineClick` — are filtered out by the `IVY_EVENT_NAMES` allow-list. Stripping `has` from every flag would advertise handlers Ivy never invokes.
- **`field` and `tooltip` serialize a singular `"child"`, not `"children"`.** Ivy has one `children?: WidgetNode[]`, so the adapter wraps the single child in a one-element array. A recursive translation reading only `children` silently drops both subtrees.
- **Enum values are a rename table, not a recasing.** Rust enums carry `#[serde(rename_all = "camelCase")]`, but several Ivy vocabularies differ by more than case: `ButtonVariant::Danger` → `"Destructive"`, `Density::{Compact,Normal,Comfortable}` → `"Small"/"Medium"/"Large"`, and `TextVariant::{Heading1,Code,Markdown}` → `"H1"/"Monospaced"/"Lead"`. Recasing is restricted to an allow-list of enum props (`variant`, `direction`, `density`, `color`, `orientation`) so user text such as `content` and `title` is never touched.

Event casing on the **inbound** side is already handled elsewhere: `EventName::canonicalize` accepts `"OnClick"`, `"onClick"` and `"click"` alike, so Ivy's PascalCase names resolve against Rust's lowercase registrations without an adapter.

**Still missing:** `src/frontend` speaks SignalR (`use-backend.tsx`), while `rusty-server` serves plain JSON WebSocket frames. That transport gap, not the node shape, is the remaining blocker to wiring the vendored frontend.
