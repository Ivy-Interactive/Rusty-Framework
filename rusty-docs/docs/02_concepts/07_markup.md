## Markup

`ivyml!` compiles declarative markup into the same builder chains you would write
by hand. There is no runtime, no interpreter and no wire-format change: the macro
expands to `Layout::vertical().gap(16.0).child(..)`, so a malformed tag is a
`rustc` error with a span rather than a panic in production.

```rust
use rusty::ivyml;

impl View for HelloApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        ivyml! {
            <Layout direction="vertical" gap=16 padding=24>
                <TextBlock content="Hello, World!" variant="heading1" />
                <TextBlock content="This is a Rusty-Framework application." />
            </Layout>
        }
    }
}
```

`ivyml` is exported from the crate root, not from the prelude — a glob-imported
`ivyml!` reads as a locally defined macro.

### Grammar

- **One root element per macro.** Wrap siblings in a container.
- **Attributes** are `name=literal` or `name={rust_expr}`.
- **Children** are nested elements or `{expr}` splices.
- Both `<Card />` and `<Card></Card>` parse; a mismatched closing tag is an error.

### Elements

Each tag maps to a constructor, because Rusty's constructors are not uniform and
cannot be derived from the tag name:

| Element | Constructor | Children attach via |
|---------|-------------|---------------------|
| `<Layout direction="vertical">` | `Layout::vertical()` | `.child()` |
| `<Layout direction="horizontal">` | `Layout::horizontal()` | `.child()` |
| `<Layout direction="grid" columns=3>` | `Layout::grid(3)` | `.child()` |
| `<TextBlock content="x">` | `TextBlock::new("x")` | — (error if given children) |
| `<Button title="x">` | `Button::new("x")` | — |
| `<Card>` | `Card::new()` | `.child()` |
| `<Container>` | `Container::new()` | `.child()` |
| `<List>` | `List::new()` | **`.item()`** |
| `<ListItem title="x">` | `ListItem::new("x")` | — |
| `<Badge label="x">` | `Badge::new("x")` | — |
| `<TextInput>` | `TextInput::new()` | — |
| `<Spacer>` | `Spacer::new()` | — |

`<List>` is why children attach through a per-element method: `List` stores
`items`, not `children`, and has no `.child` method at all.

The attributes a constructor consumes (`direction`, `columns`, `content`, `title`,
`label`) are not also emitted as builder calls. Every other attribute becomes
`.name(arg)`, with `-` mapped to `_`.

### Attribute values

Literals are coerced per slot so the markup stays free of Rust type noise:

| Attribute | Markup | Emitted |
|-----------|--------|---------|
| `gap`, `padding`, `min`, `max`, `step` | `gap=16` | `16f64` |
| `columns` | `columns=3` | `3usize` |
| `disabled`, `loading`, `wrap`, `border`, `rounded` | `disabled=true` | `true` |
| `width`, `height` | `width="100%"` / `"240px"` / `"auto"` | `Size::Percent(100.0)` / `Size::Px(240.0)` / `Size::Auto` |
| `align`, `justify` | `justify="space-between"` | `Justify::SpaceBetween` |
| `variant` on `<Button>` | `variant="ghost"` | `ButtonVariant::Ghost` |
| `variant` on `<TextBlock>` | `variant="heading1"` | `TextVariant::Heading1` |
| `on_*` | `on_click={\|\| ..}` | the closure, by value |
| anything else | `content="x"` | `"x"`, or `&(expr)` for `{expr}` |

Enum-valued attributes accept both kebab and snake spelling: `"space-between"`
and `"space_between"` both reach `Justify::SpaceBetween`.

`width`/`height` map to `Size` variants rather than to bare numbers because `Size`
is `#[serde(untagged)]` — `Px(240.0)` and `Percent(240.0)` both serialize to
`240.0`, and widgets emit `Size::to_css()` by hand. Choosing the variant at
compile time is what makes `"240px"` reach the client as `240px`.

### Interpolation

`{expr}` works in both attribute and child position. `&str` slots emit `&(expr)`,
so an interpolated `String`, `&String`, `&str` or `format!(..)` all work without
`.as_str()`:

```rust
ivyml! {
    <Layout direction="vertical" gap=8>
        <TextBlock content={format!("count = {}", count.get())} />
        <Button title="Increment" on_click={move || count.update(|v| v + 1)} />
        {existing_element}
    </Layout>
}
```

Event handlers are passed by value, never borrowed — `on_click={|| ..}` needs a
`'static` closure, and a borrowed temporary cannot satisfy that. A literal in an
`on_*` slot is an error.

Markup and builders are fully interchangeable. A `{expr}` splice accepts anything
that converts into an `Element`, including a builder chain, so you can drop into
builders for a subtree and back out again:

```rust
let rows = items
    .iter()
    .map(|i| ListItem::new(&i.name).into())
    .collect::<Vec<_>>();

ivyml! {
    <Card>
        <TextBlock content="Items" variant="heading2" />
        {List::new().items(rows)}
    </Card>
}
```

### Markup in a separate file

`ivyml_file!` compiles an external `.ivyml` file at build time. The path resolves
against `CARGO_MANIFEST_DIR`, so it is relative to the crate root rather than to
the source file:

```rust
use rusty::ivyml_file;

impl View for Dashboard {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        ivyml_file!("src/views/dashboard.ivyml")
    }
}
```

```ivyml
<Layout direction="vertical" gap=16 padding=24>
    <TextBlock content="Dashboard" variant="heading1" />
    <Layout direction="grid" columns=3 gap=12>
        <Card><TextBlock content="Requests" /></Card>
        <Card><TextBlock content="Errors" /></Card>
        <Card><TextBlock content="Latency" /></Card>
    </Layout>
</Layout>
```

`.ivyml` files reach the same parser as the inline form, which is what gives them
`{expr}` interpolation and per-token spans. The cost is that they must be
**Rust-lexable**: bare prose in child position does not lex, so use
`content="..."`.

### Diagnostics

Errors point at the offending token, not at the macro call site:

```text
error: unknown IvyML element `<Bogus>`
error: closing tag `</Card>` does not match `<Layout>`
error: unknown direction `sideways`; expected vertical, horizontal or grid
error: `<Button>` requires `title`
error: expected a f64 literal here
error: `<TextBlock>` does not accept children
error: `20em` is not a size; use `200px`, `50%` or `auto`
error: expected a single root element; wrap siblings in a container such as <Layout>
error: an event handler must be an interpolated closure, e.g. on_click={|| ..}
```
