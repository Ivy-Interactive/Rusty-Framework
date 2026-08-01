## DiffView

Renders a unified diff string — typically `git diff` output — either inline or
side by side.

### Constructor

```rust
DiffView::new().diff(patch)
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Diff | `.diff(s)` | `&str` | The unified diff text |
| View Type | `.view_type(t)` | `DiffViewType` | `Unified` (default) or `Split` |
| — | `.split()` / `.unified()` | — | Shorthand for the two view types |
| Language | `.language(s)` | `&str` | Language hint for syntax highlighting |
| Old Revision | `.old_revision(s)` | `&str` | Left-hand revision name in the header |
| New Revision | `.new_revision(s)` | `&str` | Right-hand revision name in the header |
| Word Wrap | `.word_wrap(b)` | `bool` | Wrap long lines instead of scrolling |
| Collapsible | `.collapsible(b)` | `bool` | Make the header collapse the diff |
| Default Collapsed | `.default_collapsed(b)` | `bool` | Start collapsed (needs `collapsible`) |
| On Line Click | `.on_line_click(f)` | `Fn(usize)` | Receives the clicked line number |

### Example

```rust
DiffView::new()
    .diff("@@ -1,3 +1,3 @@\n fn main() {\n-    println!(\"old\");\n+    println!(\"new\");\n }\n")
    .language("rust")
    .old_revision("HEAD~1")
    .new_revision("HEAD")
    .split()
    .collapsible(true)
    .on_line_click(|line| println!("line {line}"))
    .into()
```

### Limitations

The widget carries the diff text verbatim: parsing hunks and applying syntax
highlighting are the frontend's job, so `language` is a hint rather than a
guarantee. Nothing validates that `diff` is well-formed unified-diff output.
