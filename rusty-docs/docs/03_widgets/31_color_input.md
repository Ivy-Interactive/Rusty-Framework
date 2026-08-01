## ColorInput

A colour picker.

### Constructor

```rust
ColorInput::new()
```

Values are CSS hex strings such as `#3366ff`.

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Value | `.value(v)` | `&str` | Selected colour as hex |
| Label | `.label(l)` | `&str` | Field label |
| Disabled | `.disabled(d)` | `bool` | Disable input |

### Events

| Event | Method | Payload | Description |
|-------|--------|---------|-------------|
| Change | `.on_change(f)` | `String` | Fired with the new hex value |

### Example

```rust
let accent = use_state(ctx, "#3366ff".to_string());
let accent_set = accent.clone();

ColorInput::new()
    .label("Accent colour")
    .value(&accent.get())
    .on_change(move |value: String| {
        accent_set.set(value);
    })
    .into()
```
