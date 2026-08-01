## Terminal

An interactive terminal emulator surface.

### Constructor

```rust
Terminal::new().cols(80).rows(24)
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Cols | `.cols(n)` | `u16` | Column count; auto-sized when unset |
| Rows | `.rows(n)` | `u16` | Row count; auto-sized when unset |
| Cursor Blink | `.cursor_blink(b)` | `bool` | Blink the cursor (default `true`) |
| Cursor Style | `.cursor_style(s)` | `CursorStyle` | `Block` (default), `Underline` or `Bar` |
| Scrollback | `.scrollback(n)` | `u32` | Lines of scrollback kept (default `1000`) |
| Initial Content | `.initial_content(s)` | `&str` | Text written before the user interacts |
| Closed | `.closed(b)` | `bool` | Mark the session ended |
| Allow Clipboard | `.allow_clipboard(b)` | `bool` | Permit copy/paste (default `true`) |
| Auto Focus | `.auto_focus(b)` | `bool` | Focus on mount (default `true`) |
| Loading | `.loading(b)` | `bool` | Show the loading state |
| Loading Text | `.loading_text(s)` | `&str` | Loading message; also sets `loading` |
| Background | `.background(c)` | `Color` | Background color |
| Foreground | `.foreground(c)` | `Color` | Text color |
| On Input | `.on_input(f)` | `Fn(String)` | Receives keystrokes typed into the terminal |
| On Resize | `.on_resize(f)` | `Fn(TerminalSize)` | Receives the new `TerminalSize { cols, rows }` |
| On Link Click | `.on_link_click(f)` | `Fn(String)` | Receives the clicked URL |

### Example

```rust
Terminal::new()
    .cols(80)
    .rows(24)
    .cursor_style(CursorStyle::Bar)
    .scrollback(5000)
    .initial_content("$ echo hello\nhello\n")
    .on_input(|data| print!("{data}"))
    .on_resize(|size| println!("{}x{}", size.cols, size.rows))
    .on_link_click(|url| println!("open {url}"))
    .into()
```

### Limitations

Ivy's `Stream` property — an `IWriteStream<byte[]>` for pushing output into a
live session — is not ported, because Rusty has no stream abstraction. Content is
seeded through `initial_content` and updated by rebuilding the widget with new
content, so this suits displaying accumulated output rather than driving a
long-running interactive process.
