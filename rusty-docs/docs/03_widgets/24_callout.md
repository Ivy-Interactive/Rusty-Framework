## Callout

A highlighted block for notes, warnings and errors.

### Constructors

```rust
Callout::new()      // defaults to the Info variant
Callout::info()
Callout::success()
Callout::warning()
Callout::error()
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Title | `.title(t)` | `&str` | Bold heading above the body |
| Variant | `.variant(v)` | `CalloutVariant` | `Info`, `Success`, `Warning` or `Error` |
| Child | `.child(e)` | `impl Into<Element>` | Append one child |
| Children | `.children(v)` | `Vec<Element>` | Append several children |

### Example

```rust
Layout::vertical()
    .gap(8.0)
    .child(
        Callout::warning()
            .title("Unsaved changes")
            .child(TextBlock::paragraph("Leaving now discards your edits.")),
    )
    .child(Callout::error().child(TextBlock::paragraph("Could not reach the server.")))
    .into()
```
