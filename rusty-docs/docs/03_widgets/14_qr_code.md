## QrCode

Renders a string as a QR code.

### Constructor

```rust
QrCode::new("https://example.com")
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Value | `new(v)` / `.value(v)` | `&str` | The encoded payload |
| Pixel Size | `.pixel_size(n)` | `u32` | Size of one QR module in pixels |
| Error Correction Level | `.error_correction_level(l)` | `QrErrorCorrectionLevel` | `Low` (default), `Medium`, `Quartile` or `High` |
| Background | `.background(c)` | `Color` | Background color |
| Foreground | `.foreground(c)` | `Color` | Module color |

Higher error correction levels survive more damage to the printed code at the
cost of encoding fewer bytes in the same grid.

### Events

None — a QR code is display-only.

### Example

```rust
QrCode::new("https://example.com")
    .pixel_size(6)
    .error_correction_level(QrErrorCorrectionLevel::High)
    .background(Color::hex("#ffffff"))
    .foreground(Color::Named(NamedColor::Black))
    .into()
```

### Limitations

No encoding happens on the server: the widget ships `value` plus the rendering
parameters and the frontend generates the matrix, so no QR encoding crate is
pulled in. Nothing checks that `value` fits the chosen error correction level —
an over-long payload fails at render time rather than at build time.
