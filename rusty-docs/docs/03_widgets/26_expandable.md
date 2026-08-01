## Expandable

A collapsible section with a clickable header.

### Constructor

```rust
Expandable::new("Advanced options")
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Title | `new(title)` | `&str` | Header text |
| Expanded | `.expanded(e)` | `bool` | Whether the body is visible |
| Child | `.child(e)` | `impl Into<Element>` | Append one child |
| Children | `.children(v)` | `Vec<Element>` | Append several children |

### Events

| Event | Method | Payload | Description |
|-------|--------|---------|-------------|
| Toggle | `.on_toggle(f)` | `bool` | Fired with the requested state |

### Example

`Expandable` is controlled: it renders the `expanded` value you give it and
reports the state the user asked for, so hold the flag in state.

```rust
let open = use_state(ctx, false);
let open_val = open.get();
let open_set = open.clone();

Expandable::new("Advanced options")
    .expanded(open_val)
    .child(TextBlock::paragraph("Rarely needed settings"))
    .on_toggle(move |expanded: bool| {
        open_set.set(expanded);
    })
    .into()
```
