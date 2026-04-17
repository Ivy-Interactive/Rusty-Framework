## Prelude

The `rusty::prelude` module re-exports everything you need to build applications. Import it with:

```rust
use rusty::prelude::*;
```

### Core Types

| Type | Description |
|------|-------------|
| `Runtime` | Manages the view lifecycle and reconciliation |
| `ViewTree` | The tree structure holding views and their state |

### Traits

| Trait | Description |
|-------|-------------|
| `View` | The core trait — implement `build()` to define UI |
| `WidgetData` | Trait for serializable widget data |

### Views

| Type | Description |
|------|-------------|
| `BuildContext` | Mutable context passed to `View::build()` |
| `Element` | The element tree enum (`Widget`, `Fragment`, `Empty`) |

### Hooks

| Function | Description |
|----------|-------------|
| `use_state(ctx, init)` | Reactive state |
| `use_ref(ctx, init)` | Non-reactive mutable state |
| `use_effect(ctx, f)` | Side effect on every build |
| `use_effect_with_deps(ctx, f, deps)` | Side effect on dependency change |
| `use_memo(ctx, f, deps)` | Memoized computation |
| `use_callback(ctx, f, deps)` | Memoized closure |
| `use_reducer(ctx, reducer, init)` | Dispatch-based state |
| `use_interval(ctx, f, ms)` | Periodic timer |
| `create_context(ctx, value)` | Provide context value |
| `use_context::<T>(ctx)` | Consume context value |

### State Types

| Type | Description |
|------|-------------|
| `State<T>` | Reactive state handle (`.get()`, `.set()`, `.update()`) |
| `Ref<T>` | Non-reactive state handle |

### Widgets

| Widget | Description |
|--------|-------------|
| `Layout` | Container with vertical/horizontal/grid arrangement |
| `TextBlock` | Text display with semantic variants |
| `Button` | Clickable button |
| `Card` | Container with title and footer |
| `Dialog` | Modal overlay |
| `TextInput` | Text input field |
| `NumberInput` | Number input field |
| `Select` | Dropdown select |
| `Checkbox` | Boolean toggle |
| `Badge` | Status label |
| `Table` | Data table |
| `Progress` | Progress bar |
| `Tooltip` | Hover tooltip wrapper |

### Shared Types

| Type | Description |
|------|-------------|
| `Color` | `Named(NamedColor)`, `Hex(String)`, `Rgba { r, g, b, a }` |
| `NamedColor` | `Primary`, `Secondary`, `Success`, `Warning`, `Danger`, `Info`, `Muted`, `White`, `Black` |
| `Size` | `Px(f64)`, `Percent(f64)`, `Auto` |
| `Density` | `Compact`, `Normal`, `Comfortable` |
| `Align` | `Start`, `Center`, `End`, `Stretch` |
| `Justify` | `Start`, `Center`, `End`, `SpaceBetween`, `SpaceAround`, `SpaceEvenly` |
| `Icon` | Icon identifier (`Icon::from("name")`) |

### Server

| Type | Description |
|------|-------------|
| `RustyServer` | WebSocket server — `new(port, factory).serve().await` |
