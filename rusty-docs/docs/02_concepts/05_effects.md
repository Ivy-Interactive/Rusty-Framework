## Effects

Effects let you run side effects (logging, data fetching, timers) in response to builds and state changes.

Every effect callback returns `Option<Box<dyn FnOnce() + Send + Sync>>` — an optional cleanup function. Return `None` when there is nothing to clean up.

### use_effect

Runs once, on the first build:

```rust
use_effect(ctx, || {
    println!("View was mounted!");
    None
});
```

### use_effect_with_deps

Runs on the first build and again whenever a dependency value changes. Dependencies come **second**, the callback third:

```rust
let count = use_state(ctx, 0i32);
let count_val = count.get();

use_effect_with_deps(ctx, &[&count_val], move |deps| {
    if let Some(value) = deps[0].as_any().downcast_ref::<i32>() {
        println!("Count changed to: {}", value);
    }
    None
});
```

Deps are passed as `&[&dyn DynEq]` and compared by value. The callback receives the same slice, so it can read the values that triggered it.

### use_interval

Runs a callback periodically. The period is an `Option<Duration>`; pass `None` to pause the timer:

```rust
use std::time::Duration;

let ticks = use_state(ctx, 0u64);
let ticks_clone = ticks.clone();

use_interval(ctx, Some(Duration::from_secs(1)), move || {
    ticks_clone.update(|v| v + 1);
});
```

Unlike the other two hooks, the `use_interval` callback returns `()` — it is `Fn`, not `FnOnce`, because it runs repeatedly.

### Cleanup

Return `Some(cleanup)` to run code before the effect re-runs or when the view unmounts. The same semantics as React's `useEffect` cleanup:

```rust
use_effect(ctx, || {
    println!("subscribed");
    Some(Box::new(|| println!("unsubscribed")) as Box<dyn FnOnce() + Send + Sync>)
});
```

All of the above run side by side in the `hooks_showcase` example:

```bash
cargo run --example hooks_showcase
```
