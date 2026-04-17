## Tooltip

Wraps a child widget with a hover tooltip.

### Constructor

```rust
Tooltip::new("Tooltip text", child_element)
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| Content | `&str` | Tooltip text shown on hover |
| Child | `impl Into<Element>` | The widget to wrap |

### Example

```rust
Tooltip::new(
    "Click to submit the form",
    Button::new("Submit").variant(ButtonVariant::Primary),
)
.into()
```

### Usage Notes

- Tooltip wraps exactly one child element
- The tooltip appears on hover over the child
- Keep tooltip text concise
