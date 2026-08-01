## Form

A submittable container of labelled fields, usually built through
[`FormBuilder`](#formbuilder) rather than assembled by hand.

### Constructor

```rust
Form::new()
    .child(Field::new("Name", TextInput::new()))
    .submit_title("Save")
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Children | `.child(c)` | `impl Into<Element>` | Append a child, usually a `Field` |
| Submit Title | `.submit_title(s)` | `&str` | Submit button label |
| Disabled | `.disabled(b)` | `bool` | Disable the whole form |
| On Submit | `.on_submit(f)` | `Fn()` | Fires when the form is submitted |

## Field

Wraps one input with a label and the surrounding help, description and
validation text.

### Constructor

```rust
Field::new("Email", TextInput::new().placeholder("you@example.com"))
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Label | `new(label, child)` | `&str` | Field label |
| Child | `new(label, child)` | `impl Into<Element>` | The wrapped input |
| Description | `.description(s)` | `&str` | Text shown above the input |
| Help | `.help(s)` | `&str` | Text shown below the input |
| Required | `.required(b)` | `bool` | Mark the field required |
| Invalid | `.invalid(s)` | `&str` | Validation message; marks the field invalid |

## FormBuilder

Declares a form over a model of type `M`. Ivy binds fields with expression trees
and reflection; Rust has neither, so each field is registered with an explicit
**render closure** that receives the current model plus a `ModelSetter<M>` that
replaces it.

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Field | `.field(name, label, render)` | `FieldRender<M>` | Register a field |
| Description | `.description(name, text)` | `&str` | Set a field's description |
| Help | `.help(name, text)` | `&str` | Set a field's help text |
| Required | `.required(name)` | — | Mark a field required |
| Validate | `.validate(name, v)` | `Validator<M>` | Attach a validator to a field |
| Disabled | `.disabled(b)` | `bool` | Disable the built form |
| Submit Title | `.submit_title(s)` | `&str` | Submit button label |
| On Submit | `.on_submit(f)` | `Fn(&M)` | Receives the validated model |

Methods that take a field `name` are no-ops when no such field is registered.
Fields render in registration order — use [Layout](03_layout.md) around the form
if you need column layout.

### Validators

`rusty::views::validators` ships the common cases, each returning
`Result<(), String>`:

| Function | Rejects |
|----------|---------|
| `not_empty(value)` | Empty or whitespace-only input |
| `min_length(value, n)` | Fewer than `n` characters |
| `max_length(value, n)` | More than `n` characters |
| `email(value)` | Anything that isn't a plausible address; empty input passes |
| `url(value)` | Anything that isn't an `http`/`https` URL; empty input passes |

`email` and `url` accept empty input so that "optional but well-formed" is the
default; combine them with `not_empty` when a value is mandatory. Both are
hand-rolled, so no regex or URL crate is pulled in — they check shape, not
deliverability or reachability.

### use_form

`use_form(ctx, initial, builder)` binds a model to a builder and returns
`(State<M>, State<HashMap<String, String>>, Element)`: the model, the per-field
error map and the rendered form. Submitting runs every registered validator,
stores the resulting errors, and calls the builder's `on_submit` only when there
are none. It consumes exactly two hook slots, so hook ordering stays stable
across rebuilds.

### Example

```rust
use rusty::views::validators;
use std::sync::Arc;

#[derive(Clone, Default)]
struct Signup {
    name: String,
    email: String,
}

struct SignupForm;

impl View for SignupForm {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let builder = FormBuilder::<Signup>::new()
            .field(
                "name",
                "Name",
                Arc::new(|model: &Signup, set: ModelSetter<Signup>| {
                    let current = model.clone();
                    TextInput::new()
                        .value(&model.name)
                        .on_change(move |v| {
                            let mut next = current.clone();
                            next.name = v;
                            set(next);
                        })
                        .into()
                }),
            )
            .required("name")
            .validate("name", Arc::new(|m: &Signup| validators::not_empty(&m.name)))
            .validate("email", Arc::new(|m: &Signup| validators::email(&m.email)))
            .submit_title("Sign up")
            .on_submit(|model: &Signup| {
                println!("signed up: {}", model.name);
            });

        let (_model, _errors, form) = use_form(ctx, Signup::default(), builder);
        form
    }
}
```

### Limitations

There is no auto-scaffolding from the model type: every field needs an explicit
render closure, because Rust cannot inspect `M`'s fields at runtime the way Ivy's
expression trees do. The upside is that binding is fully type-checked — a field
that reads or writes the wrong member of `M` fails to compile.
