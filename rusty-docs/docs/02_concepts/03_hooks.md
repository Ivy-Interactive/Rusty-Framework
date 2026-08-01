## Hooks

Hooks let you add state and side effects to views. They must be called in the same order on every render — never inside conditionals or loops.

### Available Hooks

| Hook | Purpose |
|------|---------|
| `use_state` | Reactive state that triggers rebuilds |
| `use_ref` | Mutable state that does NOT trigger rebuilds |
| `use_effect` | Run side effects after every build |
| `use_effect_with_deps` | Run side effects when dependencies change |
| `use_memo` | Memoize expensive computations |
| `use_callback` | Memoize closures |
| `use_reducer` | Dispatch-based state management |
| `use_interval` | Run a callback on a timer |
| `use_context` / `create_context` | Dependency injection through the view tree |

### Usage Pattern

All hooks take `&mut BuildContext` as the first argument. `use_state` and `use_ref` take the initial *value*, not a closure that produces it:

```rust
fn build(&self, ctx: &mut BuildContext) -> Element {
    let count = use_state(ctx, 0i32);
    let name = use_ref(ctx, String::from("world"));

    // ...
}
```

Every hook in this table is demonstrated in the `hooks_showcase` example:

```bash
cargo run --example hooks_showcase
```

### Rules of Hooks

1. Only call hooks at the top level of `build()`
2. Never call hooks inside `if`, `match`, `for`, or closures
3. Hooks must be called in the same order every render
