## Card

A container with optional title, subtitle, and footer.

### Constructor

```rust
Card::new()
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Title | `.title(s)` | `&str` | Card header title |
| Subtitle | `.subtitle(s)` | `&str` | Card header subtitle |
| Padding | `.padding(n)` | `f64` | Content padding |

### Children

```rust
Card::new()
    .title("User Profile")
    .child(TextBlock::paragraph("Card content goes here"))
    .footer(Button::new("Edit"))
    .into()
```

### Example

```rust
Card::new()
    .title("Statistics")
    .subtitle("Last 30 days")
    .child(
        Layout::horizontal()
            .gap(16.0)
            .child(TextBlock::h2("1,234"))
            .child(TextBlock::label("Total views"))
    )
    .into()
```
