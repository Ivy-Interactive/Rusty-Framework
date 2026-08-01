## MultiSelect

A dropdown permitting several simultaneous selections.

### Constructor

```rust
MultiSelect::new(vec![
    SelectOption { value: "rust".into(), label: "Rust".into() },
    SelectOption { value: "ts".into(), label: "TypeScript".into() },
])
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Options | `new(options)` | `Vec<SelectOption>` | Selectable options |
| Values | `.values(v)` | `Vec<String>` | Currently selected option values |
| Label | `.label(l)` | `&str` | Field label |
| Placeholder | `.placeholder(p)` | `&str` | Hint shown when nothing is selected |
| Disabled | `.disabled(d)` | `bool` | Disable input |

### Events

| Event | Method | Payload | Description |
|-------|--------|---------|-------------|
| Change | `.on_change(f)` | `Vec<String>` | Fired with every selected value |

The handler receives the whole selection, not a delta, so the payload is
always safe to store directly. Deselecting everything yields an empty vector.

### Example

```rust
let langs = use_state(ctx, Vec::<String>::new());
let langs_set = langs.clone();

MultiSelect::new(vec![
    SelectOption { value: "rust".into(), label: "Rust".into() },
    SelectOption { value: "ts".into(), label: "TypeScript".into() },
    SelectOption { value: "go".into(), label: "Go".into() },
])
.label("Languages")
.placeholder("Pick some languages")
.values(langs.get())
.on_change(move |values: Vec<String>| {
    langs_set.set(values);
})
.into()
```
