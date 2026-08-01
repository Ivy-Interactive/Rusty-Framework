## Spacer

A flexible gap that pushes its siblings apart inside a `Layout`.

### Constructor

```rust
Spacer::new()
```

### Properties

`Spacer` has no properties — it takes whatever space the surrounding layout
gives it.

### Example

```rust
Layout::horizontal()
    .child(TextBlock::paragraph("Left"))
    .child(Spacer::new())
    .child(TextBlock::paragraph("Right"))
    .into()
```
