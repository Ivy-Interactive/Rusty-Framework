## Image

A bitmap or vector image loaded from a URL or data URI.

### Constructor

```rust
Image::new("/static/logo.png")
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Source | `new(src)` | `&str` | Image URL or data URI |
| Alt text | `.alt(a)` | `&str` | Accessible description |
| Width | `.width(s)` | `Size` | Explicit width |
| Height | `.height(s)` | `Size` | Explicit height |

### Example

```rust
Image::new("/static/avatar.png")
    .alt("Profile picture")
    .width(Size::Px(96.0))
    .height(Size::Px(96.0))
    .into()
```
