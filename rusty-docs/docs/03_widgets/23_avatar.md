## Avatar

A circular user image that falls back to initials when no image is available.

### Constructor

```rust
Avatar::new("AB")
```

The argument is the fallback text, so an avatar always renders something.

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Fallback | `new(fallback)` | `&str` | Initials shown when there is no image |
| Image | `.image(url)` | `&str` | Image URL |
| Size | `.size(d)` | `Density` | `Compact`, `Normal` or `Comfortable` |

### Example

```rust
Layout::horizontal()
    .gap(8.0)
    .child(Avatar::new("AB").image("/static/ab.png"))
    .child(Avatar::new("CD").size(Density::Compact))
    .into()
```
