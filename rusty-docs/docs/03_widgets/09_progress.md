## Progress

A progress bar widget.

### Constructors

```rust
Progress::new(0.75)          // 75% progress
Progress::indeterminate()     // Animated loading bar
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Value | `new(value)` | `f64` | Progress value (0.0 to 1.0) |
| Max | `.max(n)` | `f64` | Maximum value |
| Label | `.label(s)` | `&str` | Descriptive label |
| Color | `.color(c)` | `Color` | Bar color |

### Example

```rust
let progress = use_state(ctx, || 0.0f64);

Layout::vertical()
    .gap(8.0)
    .child(Progress::new(progress.get()).label("Upload progress"))
    .child(TextBlock::label(&format!("{:.0}%", progress.get() * 100.0)))
    .into()
```
