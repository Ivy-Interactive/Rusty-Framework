## RichTextInput

A rich text editor whose value is HTML. Named for the behaviour rather than the
JS library behind it, matching Rusty's library-agnostic widget names.

### Constructor

```rust
RichTextInput::new().value("<p>Hello</p>")
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Value | `.value(s)` | `&str` | Current content as HTML |
| Placeholder | `.placeholder(s)` | `&str` | Shown while the editor is empty |
| Disabled | `.disabled(b)` | `bool` | Disable the editor |
| Editable | `.editable(b)` | `bool` | Allow edits (default `true`) |
| — | `.read_only()` | — | Shorthand for `.editable(false)` |
| Auto Focus | `.auto_focus(b)` | `bool` | Focus on mount |
| Show Toolbar | `.show_toolbar(b)` | `bool` | Render the formatting toolbar (default `true`) |
| — | `.hide_toolbar()` | — | Shorthand for `.show_toolbar(false)` |
| Invalid | `.invalid(s)` | `&str` | Validation message; marks the input invalid |
| On Change | `.on_change(f)` | `Fn(String)` | Receives the edited HTML |
| On Focus | `.on_focus(f)` | `Fn()` | Fires when the editor gains focus |
| On Blur | `.on_blur(f)` | `Fn()` | Fires when the editor loses focus |

### Example

```rust
struct Editor;

impl View for Editor {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let html = use_state(ctx, "<p>Hello</p>".to_string());
        let html_clone = html.clone();

        RichTextInput::new()
            .value(&html.get())
            .placeholder("Write something…")
            .on_change(move |v| html_clone.set(v))
            .into()
    }
}
```

Pair it with [Field](12_form.md) to get a label and validation text, and set
`.invalid(message)` from your own validation:

```rust
Field::new("Notes", RichTextInput::new().value(&notes)).required(true)
```

### Limitations

The value is HTML produced by the frontend editor and is neither sanitized nor
validated on the server. Sanitize before storing it or rendering it anywhere
outside this widget. Ivy's `Nullable` flag is not ported — an absent value is
simply `None`.
