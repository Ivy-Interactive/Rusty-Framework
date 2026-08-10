## Toolbar

A horizontal bar of buttons, separators and grouped items.

### Constructors

```rust
Toolbar::new()
ToolbarItem::button("save")
ToolbarItem::separator()
ToolbarItem::group("Format")
```

### Toolbar properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Item | `.item(i)` | `ToolbarItem` | Append one item |
| Items | `.items(v)` | `Vec<ToolbarItem>` | Append several items |
| Disabled | `.disabled(b)` | `bool` | Disable every item |
| Density | `.density(d)` | `Density` | Button size |

### Toolbar events

| Event | Method | Payload | Description |
|-------|--------|---------|-------------|
| Select | `.on_select(f)` | `String` | Fired with the `tag` of the selected item |

### ToolbarItem properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Tag | `button(tag)` | `&str` | Identifies the item to `on_select` |
| Label | `.label(s)` | `&str` | Button text |
| Icon | `.icon(i)` | `impl Into<Icon>` | Leading icon |
| Tooltip | `.tooltip(s)` | `&str` | Hover text |
| Checked | `.checked(b)` | `bool` | Render as a toggled-on tool |
| Disabled | `.disabled(b)` | `bool` | Disable this item only |
| Item | `.item(i)` | `ToolbarItem` | Append a member to a group |

Items are properties, not widgets: they have no IDs and cannot carry closures.
One `.on_select` on the toolbar receives the tag of whichever item was clicked,
at any nesting depth. An item with no tag — a separator, or a group heading — is
inert and fires nothing.

Only a `group` renders its `children`; nesting under a button has no effect.

### Example

```rust
let selected = use_state(ctx, String::new());
let pick = selected.clone();
let bold_on = selected.get() == "bold";

Toolbar::new()
    .item(ToolbarItem::button("save").label("Save").icon("save"))
    .item(ToolbarItem::separator())
    .item(
        ToolbarItem::group("Format")
            .item(ToolbarItem::button("bold").label("Bold").checked(bold_on))
            .item(ToolbarItem::button("italic").label("Italic")),
    )
    .item(ToolbarItem::button("delete").label("Delete").disabled(true))
    .density(Density::Compact)
    .on_select(move |tag| pick.set(tag))
    .into()
```
