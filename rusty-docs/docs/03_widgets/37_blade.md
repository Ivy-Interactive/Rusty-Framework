## Blade

A horizontal stack of panels, each pushed to the right of the last. Used for
drill-down navigation: a list blade opens a detail blade beside it rather than
replacing it.

### Constructors

```rust
BladeContainer::new()
Blade::new(0)
```

### BladeContainer properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Blade | `.blade(b)` | `Blade` | Append one blade |
| Child | `.child(e)` | `impl Into<Element>` | Append any element |
| Children | `.children(v)` | `Vec<Element>` | Append several elements |

`.blade()` appends without renumbering — the index you gave each `Blade` is the
index it keeps.

### Blade properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Index | `new(index)` | `u32` | Position in the stack, 0 for the root |
| Title | `.title(s)` | `&str` | Header text |
| Width | `.width(s)` | `Size` | Panel width |
| Child | `.child(e)` | `impl Into<Element>` | Append body content |
| Children | `.children(v)` | `Vec<Element>` | Append several elements |

### Blade events

| Event | Method | Payload | Description |
|-------|--------|---------|-------------|
| Close | `.on_close(f)` | none | Fired when the close button is clicked |
| Refresh | `.on_refresh(f)` | none | Fired when the refresh button is clicked |

The close button is hidden on the blade at index `0`: the root of a stack has
nothing to return to. A blade with no `.on_close` handler renders no close
button at any index.

### Managing the stack

There is no `use_blades` hook. Which blades exist is ordinary view state, so a
drill-down is a `use_state` holding the selection and a conditional
`.blade(...)`:

### Example

```rust
let selected = use_state(ctx, None::<String>);
let pick = selected.clone();
let close = selected.clone();
let current = selected.get();

let mut container = BladeContainer::new().blade(
    Blade::new(0)
        .title("Projects")
        .width(Size::Px(240.0))
        .child(
            List::new()
                .item(ListItem::new("Rusty").on_click(move || pick.set(Some("Rusty".to_string())))),
        ),
);

if let Some(name) = current {
    container = container.blade(
        Blade::new(1)
            .title(&name)
            .child(TextBlock::paragraph("Details go here."))
            .on_close(move || close.set(None)),
    );
}

container.into()
```
