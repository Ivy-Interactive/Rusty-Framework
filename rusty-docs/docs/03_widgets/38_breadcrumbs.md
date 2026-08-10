## Breadcrumbs

A trail of links showing where the user is, with a separator between crumbs.

### Constructors

```rust
Breadcrumbs::new()
BreadcrumbItem::new("Home")
```

### Breadcrumbs properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Item | `.item(i)` | `BreadcrumbItem` | Append one crumb |
| Items | `.items(v)` | `Vec<BreadcrumbItem>` | Append several crumbs |
| Separator | `.separator(s)` | `&str` | Text between crumbs, `/` when unset |
| Disabled | `.disabled(b)` | `bool` | Disable the whole trail |
| Density | `.density(d)` | `Density` | Row height and text size |

### Breadcrumbs events

| Event | Method | Payload | Description |
|-------|--------|---------|-------------|
| Item click | `.on_item_click(f)` | `usize` | Fired with the index of the clicked crumb |

### BreadcrumbItem properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Label | `new(label)` | `&str` | Crumb text |
| Not clickable | `.not_clickable()` | — | Render as plain text instead of a link |
| Icon | `.icon(i)` | `impl Into<Icon>` | Leading icon |
| Tooltip | `.tooltip(s)` | `&str` | Hover text |
| Disabled | `.disabled(b)` | `bool` | Disable this crumb only |

A crumb is a property, not a widget: it has no ID of its own and cannot carry a
closure. One `.on_item_click` on the trail receives the index of whichever crumb
was hit. Crumbs are clickable by default; the last crumb is the current location
and is never clickable regardless.

### Example

```rust
let clicked = use_state(ctx, String::new());
let labels = vec!["Home", "Projects", "Rusty", "Widgets"];
let record = clicked.clone();
let recorded = labels.clone();

Breadcrumbs::new()
    .items(
        labels
            .iter()
            .map(|label| BreadcrumbItem::new(label))
            .collect(),
    )
    .separator(">")
    .on_item_click(move |index| record.set(recorded[index].to_string()))
    .into()
```
