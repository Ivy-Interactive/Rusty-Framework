## StackedProgress

A multi-segment progress bar where each segment carries its own value, color and label.

### Constructor

```rust
StackedProgress::new()
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Segments | `.segments(v)` / `.segment(s)` | `Vec<ProgressSegment>` | The bar's segments, in order |
| Bar height | `.bar_height(n)` | `f64` | Height of the bar in pixels |
| Show labels | `.show_labels(b)` | `bool` | Render each segment's label inline |
| Rounded | `.rounded(b)` | `bool` | Round the bar's corners |
| Selected | `.selected(i)` | `usize` | Index of the highlighted segment |
| Width | `.width(s)` | `Size` | Explicit width |
| On select | `.on_select(f)` | `Fn(usize)` | Called with the clicked segment's index |

`ProgressSegment` has its own builder:

```rust
ProgressSegment::new(3.0).label("Done").color(Color::Named(NamedColor::Success))
```

### Example

```rust
let selected = use_state(ctx, None::<usize>);
let selected_clone = selected.clone();

StackedProgress::new()
    .segment(ProgressSegment::new(3.0).label("Done").color(Color::Named(NamedColor::Success)))
    .segment(ProgressSegment::new(2.0).label("In progress"))
    .segment(ProgressSegment::new(5.0).label("Todo"))
    .show_labels(true)
    .on_select(move |index| {
        selected_clone.set(Some(index));
    })
    .into()
```
