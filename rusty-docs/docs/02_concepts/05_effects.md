## Effects

Effects let you run side effects (logging, data fetching, timers) in response to builds and state changes.

### use_effect

Runs after every build:

```rust
use_effect(ctx, || {
    println!("View was built!");
});
```

### use_effect_with_deps

Runs only when dependencies change:

```rust
let count = use_state(ctx, || 0i32);

use_effect_with_deps(ctx, {
    let current = count.get();
    move |_| {
        println!("Count changed to: {}", current);
    }
}, count.get());
```

The second argument is the effect closure. The third argument is the dependency value — the effect re-runs when this value changes (compared via `DynEq`).

### use_interval

Runs a callback periodically:

```rust
let ticks = use_state(ctx, || 0u64);
let ticks_clone = ticks.clone();

use_interval(ctx, move || {
    ticks_clone.update(|v| v + 1);
}, 1000); // every 1000ms
```

### Cleanup

Effects can return a cleanup function that runs before the next effect execution or when the view is unmounted. This pattern follows the same semantics as React's `useEffect` cleanup.
