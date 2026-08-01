## Input Widgets

Rusty-Framework provides several input widgets for forms and user interaction. All four appear together in the `form` example:

```bash
cargo run --example form
```

### TextInput

```rust
let name = use_state(ctx, String::new());
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
let age = use_state(ctx, 0.0f64);
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

let choice = use_state(ctx, String::from("a"));
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
let agreed = use_state(ctx, false);
let agreed_change = agreed.clone();

Checkbox::new(agreed.get())
    .label("I agree to the terms")
    .on_change(move |val: bool| {
        agreed_change.set(val);
    })
    .into()
```

### Read-only text

`TextInput` can be shown but not edited. Unlike `.disabled(true)`, a read-only
field stays focusable and keeps its normal styling, so it suits displaying a
generated value the user may want to copy.

```rust
TextInput::new()
    .label("API key")
    .value(&api_key)
    .read_only(true)
    .into()
```

### Specialized inputs

Six further inputs each have their own page: [TextArea](28_text_area.md),
[Slider](29_slider.md), [DateInput](30_date_input.md),
[ColorInput](31_color_input.md), [RadioGroup](32_radio_group.md) and
[MultiSelect](33_multi_select.md).
