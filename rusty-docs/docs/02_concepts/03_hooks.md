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
| `use_service` | Resolve a registered service by type |
| `use_query` | Fetch and cache data, revalidating stale entries in the background |
| `use_mutation` | Mutate, revalidate or invalidate a cached query from outside it |
| `use_signal` | Send and receive messages between views, per session or server-wide |
| `use_download` | Register a download and get the URL to serve it from |
| `use_alert` | Show a modal alert and get the user's answer in a callback |
| `use_trigger` | Render an element on demand, carrying a value to its factory |

### Usage Pattern

All hooks take `&mut BuildContext` as the first argument:

```rust
fn build(&self, ctx: &mut BuildContext) -> Element {
    let count = use_state(ctx, || 0i32);
    let name = use_ref(ctx, || String::from("world"));

    // ...
}
```

### Rules of Hooks

1. Only call hooks at the top level of `build()`
2. Never call hooks inside `if`, `match`, `for`, or closures
3. Hooks must be called in the same order every render
