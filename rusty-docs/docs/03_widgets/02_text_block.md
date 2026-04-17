## TextBlock

A text display widget with semantic variants.

### Constructors

```rust
TextBlock::new("plain text")
TextBlock::h1("Heading 1")
TextBlock::h2("Heading 2")
TextBlock::h3("Heading 3")
TextBlock::paragraph("Body text")
TextBlock::code("let x = 42;")
TextBlock::markdown("**bold** and *italic*")
TextBlock::label("Field Label")
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Content | `new(text)` | `&str` | Text content |
| Variant | `.variant(v)` | `TextVariant` | `H1`, `H2`, `H3`, `Paragraph`, `Code`, `Markdown`, `Label` |
| Color | `.color(c)` | `Color` | Text color |
| Bold | `.bold()` | — | Make text bold |
| Italic | `.italic()` | — | Make text italic |

### Example

```rust
Layout::vertical()
    .gap(8.0)
    .child(TextBlock::h1("Welcome"))
    .child(TextBlock::paragraph("This is a paragraph of text."))
    .child(TextBlock::code("println!(\"Hello!\");"))
    .into()
```
