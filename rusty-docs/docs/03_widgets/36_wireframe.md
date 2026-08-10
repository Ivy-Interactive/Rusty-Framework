## WireframeCallout

A design annotation callout pointing at the content it wraps.

### Constructor

```rust
WireframeCallout::new("Move this button up")
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Text | `new(text)` | `&str` | The annotation's message |
| Title | `.title(s)` | `&str` | Optional heading for the annotation |
| Color | `.color(c)` | `Color` | Callout color |
| Children | `.child(w)` / `.children(v)` | `Element` | Content the annotation points at |

### Example

```rust
WireframeCallout::new("Move this button up")
    .title("UX note")
    .child(Button::new("Save"))
    .into()
```

## WireframeNote

A design sticky note, optionally attributed to an author.

### Constructor

```rust
WireframeNote::new("Consider dark mode")
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Text | `new(text)` | `&str` | The note's message |
| Author | `.author(s)` | `&str` | Who left the note |
| Color | `.color(c)` | `Color` | Note color |

### Example

```rust
WireframeNote::new("Consider dark mode").author("Alex").into()
```
