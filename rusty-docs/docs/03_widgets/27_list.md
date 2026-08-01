## List

A vertical list of rows.

### Constructors

```rust
List::new()
ListItem::new("Inbox")
```

### List properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Item | `.item(e)` | `impl Into<Element>` | Append one item |
| Items | `.items(v)` | `Vec<Element>` | Append several items |

A `List` accepts any element, not just `ListItem`.

### ListItem properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Title | `new(title)` | `&str` | Primary text |
| Subtitle | `.subtitle(s)` | `&str` | Secondary text |
| Icon | `.icon(i)` | `impl Into<Icon>` | Leading icon |

### ListItem events

| Event | Method | Payload | Description |
|-------|--------|---------|-------------|
| Click | `.on_click(f)` | none | Fired when the row is clicked |

Each item is a widget in its own right, so it gets its own ID and its own
handler — the click tells you which row was hit without an index.

### Example

```rust
let selected = use_state(ctx, String::new());
let pick_inbox = selected.clone();
let pick_drafts = selected.clone();

List::new()
    .item(
        ListItem::new("Inbox")
            .subtitle("3 unread")
            .icon("mail")
            .on_click(move || pick_inbox.set("inbox".to_string())),
    )
    .item(ListItem::new("Drafts").on_click(move || pick_drafts.set("drafts".to_string())))
    .item(ListItem::new("Archive"))
    .into()
```
