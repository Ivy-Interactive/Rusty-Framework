## Dialog

A modal dialog overlay.

### Constructor

```rust
Dialog::new(is_open)
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Open | `new(open)` | `bool` | Controls visibility |
| Title | `.title(s)` | `&str` | Dialog header |

### Children

```rust
Dialog::new(true)
    .title("Confirm Delete")
    .child(TextBlock::paragraph("Are you sure?"))
    .footer(
        Layout::horizontal()
            .gap(8.0)
            .child(Button::new("Cancel"))
            .child(Button::new("Delete").color(Color::Named(NamedColor::Danger)))
    )
    .into()
```

### Example with State

```rust
let open = use_state(ctx, || false);
let open_show = open.clone();
let open_hide = open.clone();

Layout::vertical()
    .child(
        Button::new("Open Dialog")
            .on_click(move || open_show.set(true))
    )
    .child(
        Dialog::new(open.get())
            .title("My Dialog")
            .child(TextBlock::paragraph("Dialog content"))
            .footer(
                Button::new("Close")
                    .on_click(move || open_hide.set(false))
            )
    )
    .into()
```
