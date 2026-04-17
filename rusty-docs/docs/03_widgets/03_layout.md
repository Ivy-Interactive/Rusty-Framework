## Layout

A container widget that arranges children in vertical, horizontal, or grid arrangements.

### Constructors

```rust
Layout::vertical()    // Stack top-to-bottom
Layout::horizontal()  // Arrange left-to-right
Layout::grid(columns) // Grid with N columns
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Gap | `.gap(n)` | `f64` | Space between children |
| Padding | `.padding(n)` | `f64` | Inner padding |
| Align | `.align(a)` | `Align` | Cross-axis alignment |
| Justify | `.justify(j)` | `Justify` | Main-axis justification |

### Children

```rust
Layout::vertical()
    .child(widget1)          // Add single child
    .children(vec![w1, w2])  // Add multiple children
    .into()
```

### Example

```rust
Layout::horizontal()
    .gap(8.0)
    .align(Align::Center)
    .child(Button::new("Cancel").variant(ButtonVariant::Ghost))
    .child(Button::new("Save").variant(ButtonVariant::Primary))
    .into()
```
