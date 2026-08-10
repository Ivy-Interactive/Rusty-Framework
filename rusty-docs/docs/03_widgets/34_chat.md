## Chat

A conversation surface: a scrolling list of messages with a composer beneath it.
Four widgets work together — `Chat` holds the thread, `ChatMessage` is one
bubble, and `ChatLoading` and `ChatStatus` report what the assistant is doing
between messages.

### Constructor

```rust
Chat::new().placeholder("Ask something…")
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Messages | `.message(e)` | `impl Into<Element>` | Append one message |
| — | `.messages(v)` | `Vec<Element>` | Append several |
| Placeholder | `.placeholder(s)` | `&str` | Composer placeholder |
| Streaming | `.streaming(b)` | `bool` | Swap the send button for a cancel button |
| Quick Replies | `.quick_reply(s)` | `&str` | Add one suggested reply |
| — | `.quick_replies(v)` | `Vec<String>` | Add several |
| Width | `.width(s)` | `Size` | Surface width |
| Height | `.height(s)` | `Size` | Surface height |
| Density | `.density(d)` | `Density` | Spacing |
| On Send | `.on_send(f)` | `Fn(String)` | Receives the submitted text |
| On Cancel | `.on_cancel(f)` | `Fn()` | Fires when a streaming response is interrupted |

### ChatMessage

```rust
ChatMessage::user(TextBlock::paragraph("What is Rusty?"))
ChatMessage::assistant(TextBlock::paragraph("A Rust UI framework."))
```

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Sender | `.sender(s)` | `ChatSender` | `User` or `Assistant` (default `User`) |
| Children | `.child(e)` | `impl Into<Element>` | Append body content |
| — | `.children(v)` | `Vec<Element>` | Append several |

The body is arbitrary widgets, not just text — put a `Card`, a `DataTable` or a
`ChatLoading` in a bubble and it renders there.

### ChatLoading and ChatStatus

`ChatLoading::new()` is a typing indicator and takes no properties.
`ChatStatus::new(text)` is a single line of status text, settable with
`.text(s)`. Both are usually the body of an assistant message:

```rust
ChatMessage::assistant(ChatStatus::new("Searching the docs…"))
ChatMessage::assistant(ChatLoading::new())
```

### Example

```rust
struct Conversation;

impl View for Conversation {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let history = use_state(ctx, Vec::<String>::new());
        let sent = history.get();
        let history_clone = history.clone();

        let bubbles: Vec<Element> = sent
            .iter()
            .map(|text| ChatMessage::user(TextBlock::paragraph(text)).into())
            .collect();

        Chat::new()
            .messages(bubbles)
            .placeholder("Ask something…")
            .quick_reply("Summarise this")
            .on_send(move |text| {
                history_clone.update(|h| {
                    let mut next = h.clone();
                    next.push(text.clone());
                    next
                })
            })
            .into()
    }
}
```

### Notes

`quick_replies` is Rust-side only: selecting a quick reply fires `on_send` with
the reply text, because the payload is exactly what typing it would produce. The
Ivy React `ChatWidget` reads no such property, so nothing renders the buttons
there yet — the E2E harness does.

Rusty does not stream tokens into a message for you. `.streaming(true)` only
changes the composer's affordance; producing the partial text is your app's job,
usually by rewriting the last `ChatMessage` as chunks arrive.
