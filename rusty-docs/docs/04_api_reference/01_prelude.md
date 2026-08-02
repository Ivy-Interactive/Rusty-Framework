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

`View` is the only trait the prelude exports. The two widget traits are not in it and need explicit imports:

```rust
use rusty::views::Widget;           // trait: widget_type / to_json / as_any
use rusty::views::view::WidgetData; // trait: the type-erased form stored in Element
use rusty::Widget as WidgetDerive;  // #[derive(Widget)] macro, same name as the trait
```

### Views

| Type | Description |
|------|-------------|
| `BuildContext` | Mutable context passed to `View::build()` |
| `Element` | The element tree enum (`Widget`, `Fragment`, `Empty`) |

### Hooks

| Function | Description |
|----------|-------------|
| `use_state(ctx, init)` | Reactive state, from an initial value |
| `use_ref(ctx, init)` | Non-reactive mutable state, from an initial value |
| `use_effect(ctx, f)` | Side effect on mount |
| `use_effect_with_deps(ctx, deps, f)` | Side effect on dependency change |
| `use_memo(ctx, deps, compute)` | Memoized computation |
| `use_callback(ctx, deps, f)` | Memoized closure |
| `use_reducer(ctx, reducer, init)` | Dispatch-based state |
| `use_interval(ctx, Some(duration), f)` | Periodic timer; `None` pauses it |
| `create_context(ctx, value)` | Provide context value |
| `use_context::<T>(ctx)` | Consume context value |

Note the argument order: wherever a hook takes dependencies, they come **before** the closure.

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

### Filters

Re-exported from `rusty_filter`. See [Filters](../02_concepts/07_filters.md).

| Item | Description |
|------|-------------|
| `parse_query(q, &columns)` | Parse and validate, returning `ParseResult` |
| `parse_query_unchecked(q)` | Parse without a column schema |
| `evaluate(&filter, &row, &columns)` | Does one row match? |
| `retain_matching(&filter, rows, &columns)` | Keep the matching rows |
| `count_matches(&filter, &rows, &columns)` | Count the matching rows |
| `validate_filter_group(&filter, &columns)` | Semantic errors only |
| `to_query_string(&filter)` | Print an AST back to a query |
| `canonical_key(&filter)` | The same string, named for cache-key use |
| `ColumnDef` | One filterable column — `new(name, ColumnType)` |
| `ColumnType` | `String`, `Number`, `Boolean`, `Date`, `Enum` |
| `FilterGroup` | `op` plus a list of `Filter` |
| `Filter` | A `Condition` or a nested `FilterGroup`, optionally negated |
| `Condition` | `column`, `function`, `args` |
| `FilterFunction` | `Equals`, `GreaterThan`, `Contains`, `IsBlank`, … |
| `LogicalOp` | `And`, `Or` |
| `ParseResult` | `filters` **or** `errors`, never both |
| `ParseError` | `message`, `start`, `end`, `severity` |
| `ErrorSeverity` | `Error`, `Warning` |

The crate itself is re-exported too, so `rusty_filter::lexer` and the other
modules are reachable without adding a second dependency.
