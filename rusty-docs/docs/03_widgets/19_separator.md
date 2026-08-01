## Separator

A dividing rule, optionally labelled with inline text.

### Constructors

```rust
Separator::horizontal()
Separator::vertical()
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Orientation | `horizontal()` / `vertical()` | `Orientation` | Axis of the rule |
| Text | `.text(t)` | `&str` | Inline label drawn over the rule |

### Example

```rust
Layout::vertical()
    .gap(8.0)
    .child(Button::new("Sign in"))
    .child(Separator::horizontal().text("OR"))
    .child(Button::new("Create account"))
    .into()
```

A vertical separator divides a horizontal row:

```rust
Layout::horizontal()
    .gap(8.0)
    .child(TextBlock::paragraph("Drafts"))
    .child(Separator::vertical())
    .child(TextBlock::paragraph("Sent"))
    .into()
```
