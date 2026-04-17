## State

State management in Rusty-Framework is hook-based. The primary hook is `use_state`.

### use_state

```rust
let count = use_state(ctx, || 0i32);
```

Returns a `State<T>` handle that is `Clone`, `Send`, and `Sync`.

### Reading State

```rust
let value = count.get(); // Returns a copy of the current value
```

### Writing State

```rust
count.set(42);              // Replace the value
count.update(|v| v + 1);   // Update based on current value
```

Both `.set()` and `.update()` trigger a rebuild of the owning view.

### Sharing State with Closures

`State<T>` is cheaply cloneable. Clone it before moving into event closures:

```rust
let count = use_state(ctx, || 0i32);
let count_for_click = count.clone();

Button::new(format!("Count: {}", count.get()))
    .on_click(move || {
        count_for_click.update(|v| v + 1);
    })
    .into()
```

### use_ref (Non-Reactive State)

For state that should NOT trigger rebuilds:

```rust
let render_count = use_ref(ctx, || 0u32);
render_count.update(|v| v + 1);
```

`Ref<T>` has the same API as `State<T>` but mutations are silent.

### use_reducer

For complex state with many transitions:

```rust
fn reducer(state: &MyState, action: MyAction) -> MyState {
    match action {
        MyAction::Increment => MyState { count: state.count + 1 },
        MyAction::Reset => MyState { count: 0 },
    }
}

let (state, dispatch) = use_reducer(ctx, reducer, MyState { count: 0 });
```
