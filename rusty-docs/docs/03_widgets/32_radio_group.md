## RadioGroup

A set of mutually exclusive radio buttons. Prefer it over `Select` when the
options are few and worth showing at once.

### Constructor

```rust
RadioGroup::new(vec![
    SelectOption { value: "s".into(), label: "Small".into() },
    SelectOption { value: "l".into(), label: "Large".into() },
])
```

`RadioGroup` shares `SelectOption` with `Select`, so option lists move between
the two unchanged.

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Options | `new(options)` | `Vec<SelectOption>` | Selectable options |
| Value | `.value(v)` | `&str` | Currently selected option value |
| Label | `.label(l)` | `&str` | Group label |
| Orientation | `.orientation(o)` | `Orientation` | Stack vertically (default) or in a row |
| Disabled | `.disabled(d)` | `bool` | Disable every option |

### Events

| Event | Method | Payload | Description |
|-------|--------|---------|-------------|
| Change | `.on_change(f)` | `String` | Fired with the newly selected value |

### Example

```rust
let size = use_state(ctx, "m".to_string());
let size_set = size.clone();

RadioGroup::new(vec![
    SelectOption { value: "s".into(), label: "Small".into() },
    SelectOption { value: "m".into(), label: "Medium".into() },
    SelectOption { value: "l".into(), label: "Large".into() },
])
.label("Size")
.orientation(Orientation::Horizontal)
.value(&size.get())
.on_change(move |value: String| {
    size_set.set(value);
})
.into()
```
