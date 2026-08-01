## Slider

A numeric slider bounded by `min` and `max`.

### Constructor

```rust
Slider::new(50.0)
```

The argument is the current value. `min` and `max` default to `0.0` and
`100.0`, so a bare `Slider::new(v)` behaves as a percentage.

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Value | `new(value)` | `f64` | Current value |
| Minimum | `.min(m)` | `f64` | Lower bound (default `0.0`) |
| Maximum | `.max(m)` | `f64` | Upper bound (default `100.0`) |
| Step | `.step(s)` | `f64` | Increment between stops |
| Label | `.label(l)` | `&str` | Field label |
| Disabled | `.disabled(d)` | `bool` | Disable input |

### Events

| Event | Method | Payload | Description |
|-------|--------|---------|-------------|
| Change | `.on_change(f)` | `f64` | Fired as the handle moves |

### Example

```rust
let volume = use_state(ctx, 25.0f64);
let volume_set = volume.clone();

Slider::new(volume.get())
    .label("Volume")
    .min(0.0)
    .max(100.0)
    .step(5.0)
    .on_change(move |value: f64| {
        volume_set.set(value);
    })
    .into()
```
