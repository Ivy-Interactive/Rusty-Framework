## TextArea

A multi-line text input.

### Constructor

```rust
TextArea::new()
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Value | `.value(v)` | `&str` | Current text |
| Placeholder | `.placeholder(p)` | `&str` | Hint shown when empty |
| Label | `.label(l)` | `&str` | Field label |
| Rows | `.rows(r)` | `usize` | Visible height in text rows |
| Disabled | `.disabled(d)` | `bool` | Disable input |

### Events

| Event | Method | Payload | Description |
|-------|--------|---------|-------------|
| Change | `.on_change(f)` | `String` | Fired as the user types |

### Example

```rust
let body = use_state(ctx, String::new());
let body_set = body.clone();

TextArea::new()
    .label("Message")
    .placeholder("Say something")
    .rows(6)
    .value(&body.get())
    .on_change(move |value: String| {
        body_set.set(value);
    })
    .into()
```
