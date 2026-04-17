## Badge

A small label for status indicators and tags.

### Constructor

```rust
Badge::new("Active")
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Label | `new(label)` | `&str` | Badge text |
| Variant | `.variant(v)` | `BadgeVariant` | Visual style |
| Color | `.color(c)` | `Color` | Badge color |

### Example

```rust
Layout::horizontal()
    .gap(8.0)
    .child(Badge::new("Active").color(Color::Named(NamedColor::Success)))
    .child(Badge::new("Pending").color(Color::Named(NamedColor::Warning)))
    .child(Badge::new("Error").color(Color::Named(NamedColor::Danger)))
    .into()
```
