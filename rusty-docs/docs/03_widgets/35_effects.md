## Confetti

A one-shot celebration effect wrapping its children.

### Constructor

```rust
Confetti::new()
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Trigger | `.trigger(t)` | `EffectTrigger` | What causes the effect to play: `Auto` (default), `Click`, `Hover` |
| Children | `.child(w)` / `.children(v)` | `Element` | Content the effect wraps |

### Example

```rust
Confetti::new()
    .trigger(EffectTrigger::Click)
    .child(Button::new("Celebrate"))
    .into()
```

## Animation

A Lottie/CSS animation wrapper around its children.

### Constructor

```rust
Animation::new()
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Animation type | `.animation_type(t)` | `AnimationType` | Kind of motion, e.g. `Rotate` (default), `SlideIn`, `Bounce`, `Shake` |
| Duration | `.duration(n)` | `f64` | Seconds the animation takes to play |
| Delay | `.delay(n)` | `f64` | Seconds before the animation starts |
| Direction | `.direction(d)` | `AnimationDirection` | `Left`, `Right`, `Up`, `Down` |
| Distance | `.distance(n)` | `f64` | Pixels travelled for slide-style animations |
| Easing | `.easing(e)` | `AnimationEasing` | Easing curve, e.g. `Linear` (default), `EaseInOut`, `BounceOut` |
| Repeat | `.repeat(n)` | `i32` | Number of times to repeat |
| Repeat delay | `.repeat_delay(n)` | `f64` | Seconds between repeats |
| Visible | `.visible(b)` | `bool` | Whether the wrapped content is currently shown |
| Intensity | `.intensity(n)` | `f64` | Strength multiplier for effects like `Shake` |
| Trigger | `.trigger(t)` | `EffectTrigger` | What starts the animation |
| Children | `.child(w)` / `.children(v)` | `Element` | Content the animation wraps |

The animation kind is exposed as `animation_type` (serialized as `animationType`) rather
than `type`, since `type` is reserved for the widget discriminator.

### Example

```rust
Animation::new()
    .animation_type(AnimationType::Bounce)
    .easing(AnimationEasing::EaseInOut)
    .duration(1.0)
    .child(TextBlock::paragraph("Bouncing"))
    .into()
```
