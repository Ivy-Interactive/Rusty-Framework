## Media Inputs

Three inputs that capture from device hardware: `AudioInput` records from the
microphone, `CameraInput` takes a photo or clip, and `SignatureInput` collects a
pointer-drawn signature. All three deliver their result as a base64 data URL.

### AudioInput

```rust
AudioInput::new().label("Record a note")
```

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Label | `.label(s)` | `&str` | Button label when idle |
| Recording Label | `.recording_label(s)` | `&str` | Button label while recording |
| Mime Type | `.mime_type(s)` | `&str` | Container format (default `"audio/webm"`) |
| Disabled | `.disabled(b)` | `bool` | Disable the control |
| Show Waveform | `.show_waveform(b)` | `bool` | Draw a live level meter (default `true`) |
| Upload Url | `.upload_url(s)` | `&str` | POST the recording here instead of firing `on_capture` |
| Chunk Interval | `.chunk_interval(n)` | `u32` | Milliseconds between recorder chunks (default `1000`) |
| Sample Rate | `.sample_rate(n)` | `u32` | Requested capture rate in Hz |
| Width | `.width(s)` | `Size` | Control width |
| Invalid | `.invalid(s)` | `&str` | Validation message; marks the input invalid |
| Auto Focus | `.auto_focus(b)` | `bool` | Focus on mount |
| On Capture | `.on_capture(f)` | `Fn(String)` | Receives the recording as a data URL |
| On Focus | `.on_focus(f)` | `Fn()` | Fires on focus |
| On Blur | `.on_blur(f)` | `Fn()` | Fires on blur |

### CameraInput

```rust
CameraInput::new().facing_mode(FacingMode::Environment)
```

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Placeholder | `.placeholder(s)` | `&str` | Shown before the preview starts |
| Disabled | `.disabled(b)` | `bool` | Disable the shutter |
| Upload Url | `.upload_url(s)` | `&str` | POST the capture here instead of firing `on_capture` |
| Facing Mode | `.facing_mode(m)` | `FacingMode` | `User` (front) or `Environment` (rear); default `User` |
| Capture Mode | `.capture_mode(m)` | `CaptureMode` | `Image` or `Video`; default `Image` |
| Width | `.width(s)` | `Size` | Control width |
| Invalid | `.invalid(s)` | `&str` | Validation message; marks the input invalid |
| Auto Focus | `.auto_focus(b)` | `bool` | Focus on mount |
| On Capture | `.on_capture(f)` | `Fn(String)` | Receives the capture as a data URL |
| On Focus | `.on_focus(f)` | `Fn()` | Fires on focus |
| On Blur | `.on_blur(f)` | `Fn()` | Fires on blur |

### SignatureInput

```rust
SignatureInput::new().placeholder("Sign here")
```

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Value | `.value(s)` | `&str` | Existing signature as a base64 PNG data URL |
| Disabled | `.disabled(b)` | `bool` | Disable drawing |
| Invalid | `.invalid(s)` | `&str` | Validation message; marks the input invalid |
| Placeholder | `.placeholder(s)` | `&str` | Hint shown on the empty pad |
| Pen | `.pen(c)` | `Color` | Ink colour |
| Background | `.background(c)` | `Color` | Pad background |
| Pen Thickness | `.pen_thickness(n)` | `f64` | Stroke width in CSS pixels |
| Density | `.density(d)` | `Density` | Spacing |
| Auto Focus | `.auto_focus(b)` | `bool` | Focus on mount |
| On Sign | `.on_sign(f)` | `Fn(String)` | Receives the signature as a base64 PNG |
| On Clear | `.on_clear(f)` | `Fn()` | Fires when the pad is wiped |
| On Focus | `.on_focus(f)` | `Fn()` | Fires on focus |
| On Blur | `.on_blur(f)` | `Fn()` | Fires on blur |

### Example

```rust
struct CaptureForm;

impl View for CaptureForm {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let signature = use_state(ctx, String::new());
        let sign = signature.clone();
        let clear = signature.clone();

        Layout::vertical()
            .gap(16.0)
            .child(
                AudioInput::new()
                    .label("Record a note")
                    .recording_label("Stop")
                    .on_capture(|data_url| println!("{} bytes of audio", data_url.len())),
            )
            .child(
                SignatureInput::new()
                    .placeholder("Sign here")
                    .pen_thickness(2.0)
                    .on_sign(move |png| sign.set(png))
                    .on_clear(move || clear.set(String::new())),
            )
            .into()
    }
}
```

Pair any of them with [Field](12_form.md) for a label and validation text.

### Two ways to receive a capture

`on_capture` is the Rust-native path: the recording arrives over the event socket
as a data URL, with no HTTP upload. It is the only path that works out of the
box, because Rusty ships no upload endpoint.

`upload_url` matches what Ivy's React widgets do instead: they POST the recording
to that URL themselves and fire no capture event at all. Set it when you already
have a service to receive the bytes — large recordings are better off not
travelling through the event socket.

### Notes

`AudioInput::show_waveform` and `CameraInput::capture_mode` have no Ivy
counterpart: the React `AudioInputWidget` always draws its waveform and
`CameraInputWidget` always takes stills. Both properties are serialized, so a
client that wants to honour them can.

`FacingMode` reaches the client lowercase (`"user"` / `"environment"`) rather than
title-cased like Rusty's other enum properties, because the value is passed
straight to `getUserMedia`, which accepts nothing else.

The captured data URL is neither decoded nor validated on the server. Check the
size and the declared mime type before storing it.
