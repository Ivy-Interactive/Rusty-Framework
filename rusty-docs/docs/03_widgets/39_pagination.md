## Pagination

A page selector: previous and next arrows around a run of page numbers, with
ellipsis gaps when there are more pages than fit.

### Constructors

```rust
Pagination::new(1, 10)
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Page | `new(page, _)` | `u32` | Selected page, 1-based |
| Page count | `new(_, num_pages)` | `u32` | Total number of pages |
| Siblings | `.siblings(n)` | `u32` | Pages shown either side of the current one, default `1` |
| Boundaries | `.boundaries(n)` | `u32` | Pages always shown at each end, default `1` |
| Disabled | `.disabled(b)` | `bool` | Disable every control |
| Density | `.density(d)` | `Density` | Control size |

`page` is 1-based: page `1` disables the previous arrow, page `num_pages`
disables the next one. Pass `0` for "no page selected" — both arrows and the
sibling pages go inert, leaving only the boundary pages.

### Events

| Event | Method | Payload | Description |
|-------|--------|---------|-------------|
| Change | `.on_change(f)` | `u32` | Fired with the newly selected 1-based page |

The widget does not hold the page itself. `on_change` reports the requested page
and the view supplies the new `page` on the next build, so a handler that does
not store the value leaves the selection where it was.

### Example

```rust
let page = use_state(ctx, 1u32);
let current = page.get();
let select = page.clone();

Layout::vertical()
    .gap(16.0)
    .child(TextBlock::paragraph(&format!("Showing page {}", current)))
    .child(
        Pagination::new(current, 10)
            .siblings(1)
            .boundaries(1)
            .on_change(move |next| select.set(next)),
    )
    .into()
```
