## DateInput

A date picker.

### Constructor

```rust
DateInput::new()
```

Dates are ISO-8601 `YYYY-MM-DD` strings rather than a date type, which keeps
the framework free of a calendar dependency. Parse them with whichever crate
your application already uses.

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Value | `.value(v)` | `&str` | Selected date, `YYYY-MM-DD` |
| Label | `.label(l)` | `&str` | Field label |
| Minimum | `.min(m)` | `&str` | Earliest selectable date |
| Maximum | `.max(m)` | `&str` | Latest selectable date |
| Disabled | `.disabled(d)` | `bool` | Disable input |

### Events

| Event | Method | Payload | Description |
|-------|--------|---------|-------------|
| Change | `.on_change(f)` | `String` | Fired with the new `YYYY-MM-DD` value |

### Example

```rust
let due = use_state(ctx, String::new());
let due_set = due.clone();

DateInput::new()
    .label("Due date")
    .min("2026-01-01")
    .max("2026-12-31")
    .value(&due.get())
    .on_change(move |value: String| {
        due_set.set(value);
    })
    .into()
```
