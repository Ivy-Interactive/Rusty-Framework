## Skeleton

A grey placeholder block shown while real content loads.

### Constructor

```rust
Skeleton::new()
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Width | `.width(s)` | `Size` | Explicit width |
| Height | `.height(s)` | `Size` | Explicit height |

### Example

Pair it with `use_query`'s loading flag:

```rust
let result = use_query(ctx, Some("profile"), fetch_profile, QueryOptions::default());

if result.loading {
    Layout::vertical()
        .gap(8.0)
        .child(Skeleton::new().width(Size::Percent(60.0)).height(Size::Px(20.0)))
        .child(Skeleton::new().width(Size::Percent(90.0)).height(Size::Px(16.0)))
        .into()
} else {
    TextBlock::paragraph(&result.value.unwrap_or_default()).into()
}
```
