## Button

An interactive button widget.

### Constructor

```rust
Button::new("Click me")
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Title | `new(title)` | `&str` | Button label text |
| Variant | `.variant(v)` | `ButtonVariant` | `Default`, `Primary`, `Ghost` |
| Icon | `.icon(i)` | `Icon` | Optional leading icon |
| Color | `.color(c)` | `Color` | Button color |
| Density | `.density(d)` | `Density` | `Compact`, `Normal`, `Comfortable` |
| Disabled | `.disabled(b)` | `bool` | Disable interaction |
| Loading | `.loading(b)` | `bool` | Show loading spinner |

### Events

| Event | Method | Signature |
|-------|--------|-----------|
| Click | `.on_click(f)` | `Fn() + Send + Sync` |

### Example

```rust
let count = use_state(ctx, || 0i32);
let count_click = count.clone();

Button::new(format!("Clicked {} times", count.get()))
    .variant(ButtonVariant::Primary)
    .on_click(move || {
        count_click.update(|v| v + 1);
    })
    .into()
```
