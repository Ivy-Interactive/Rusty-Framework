## Layout

Layout widgets control how children are arranged on screen.

### Vertical Layout

Stack children top-to-bottom:

```rust
Layout::vertical()
    .gap(16.0)
    .padding(24.0)
    .child(TextBlock::h1("Title"))
    .child(TextBlock::paragraph("Body text"))
    .into()
```

### Horizontal Layout

Arrange children left-to-right:

```rust
Layout::horizontal()
    .gap(8.0)
    .align(Align::Center)
    .child(Button::new("Cancel"))
    .child(Button::new("OK"))
    .into()
```

### Grid Layout

Arrange children in a grid with a fixed number of columns:

```rust
Layout::grid(3)
    .gap(12.0)
    .children(items.iter().map(|item| {
        Card::new().title(&item.name).into()
    }).collect())
    .into()
```

### Alignment and Justification

- `.align(Align)` — cross-axis alignment: `Start`, `Center`, `End`, `Stretch`
- `.justify(Justify)` — main-axis justification: `Start`, `Center`, `End`, `SpaceBetween`, `SpaceAround`, `SpaceEvenly`

### Spacing

- `.gap(f64)` — space between children
- `.padding(f64)` — inner padding on all sides
