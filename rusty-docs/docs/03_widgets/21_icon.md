## Icon

A named icon. The widget struct is `IconWidget`, because `Icon` is the newtype
holding the name it accepts.

### Constructor

```rust
IconWidget::new("check")
```

`new` takes `impl Into<Icon>`, so a `&str` or an `Icon` both work.

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Name | `new(name)` | `impl Into<Icon>` | Icon name |
| Size | `.size(s)` | `f64` | Rendered size in pixels |
| Color | `.color(c)` | `Color` | Icon colour |

### Example

```rust
Layout::horizontal()
    .gap(8.0)
    .child(IconWidget::new("check").color(Color::Named(NamedColor::Success)))
    .child(IconWidget::new("alert").size(24.0))
    .into()
```
