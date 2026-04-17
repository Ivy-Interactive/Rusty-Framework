## Input Widgets

Rusty-Framework provides several input widgets for forms and user interaction.

### TextInput

```rust
let name = use_state(ctx, || String::new());
let name_change = name.clone();

TextInput::new()
    .value(&name.get())
    .label("Name")
    .placeholder("Enter your name")
    .on_change(move |val: String| {
        name_change.set(val);
    })
    .into()
```

### NumberInput

```rust
let age = use_state(ctx, || 0.0f64);
let age_change = age.clone();

NumberInput::new()
    .value(age.get())
    .label("Age")
    .min(0.0)
    .max(150.0)
    .step(1.0)
    .on_change(move |val: f64| {
        age_change.set(val);
    })
    .into()
```

### Select

```rust
use rusty::widgets::input::SelectOption;

let choice = use_state(ctx, || String::from("a"));
let choice_change = choice.clone();

Select::new(vec![
    SelectOption { value: "a".into(), label: "Option A".into() },
    SelectOption { value: "b".into(), label: "Option B".into() },
])
.value(&choice.get())
.label("Choose one")
.on_change(move |val: String| {
    choice_change.set(val);
})
.into()
```

### Checkbox

```rust
let agreed = use_state(ctx, || false);
let agreed_change = agreed.clone();

Checkbox::new(agreed.get())
    .label("I agree to the terms")
    .on_change(move |val: bool| {
        agreed_change.set(val);
    })
    .into()
```
