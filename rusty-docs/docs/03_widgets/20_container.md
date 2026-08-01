## Container

A single box that pads, sizes and decorates whatever it wraps. Reach for
`Container` when you want a background or border without `Card`'s title and
footer structure.

### Constructor

```rust
Container::new()
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Child | `.child(e)` | `impl Into<Element>` | Append one child |
| Children | `.children(v)` | `Vec<Element>` | Append several children |
| Padding | `.padding(p)` | `f64` | Inner padding in pixels |
| Width | `.width(s)` | `Size` | Explicit width |
| Height | `.height(s)` | `Size` | Explicit height |
| Background | `.background(c)` | `Color` | Fill colour |
| Border | `.border(b)` | `bool` | Draw a one-pixel border |
| Rounded | `.rounded(r)` | `bool` | Round the corners |

### Example

```rust
Container::new()
    .padding(16.0)
    .width(Size::Percent(100.0))
    .background(Color::Named(NamedColor::Muted))
    .border(true)
    .rounded(true)
    .child(TextBlock::paragraph("Wrapped content"))
    .into()
```
