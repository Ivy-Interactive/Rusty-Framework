## Views

Views are the fundamental building blocks of a Rusty-Framework application. A view is any type that implements the `View` trait.

### The View Trait

```rust
pub trait View: Send + Sync + 'static {
    fn build(&self, ctx: &mut BuildContext) -> Element;
}
```

### BuildContext

`BuildContext` is the mutable context passed to every `build()` call. It provides:

- **Hook management** — `next_hook_index()` for ordered hook calls
- **Event registration** — `register_event()` to bind handlers to widget events
- **Widget IDs** — `next_widget_id()` generates unique IDs
- **Child views** — `child_view()` embeds sub-views with isolated hook stores
- **Context propagation** — `find_ancestor_context()` for dependency injection

### Lifecycle

1. The server creates a `Runtime` per client connection
2. On connect, `build()` is called to produce the initial widget tree
3. When state changes, `build()` is called again
4. The framework diffs the old and new trees and sends only changes to the client

### Closures as Views

Any closure matching `Fn(&mut BuildContext) -> Element + Send + Sync + 'static` also implements `View`:

```rust
let my_view = |ctx: &mut BuildContext| -> Element {
    TextBlock::paragraph("I'm a closure view!").into()
};
```

### Child Views

Use `ctx.child_view()` to embed a child view with its own isolated hook store. It returns a 2-tuple of `(Element, ViewId)`, so destructure it rather than passing the result straight to `.child()`:

```rust
impl View for ParentView {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let (content, _view_id) = ctx.child_view(ChildView);

        Layout::vertical().child(content).into()
    }
}
```

The parent persists the child's `HookStore` automatically, keyed by the child's `ViewId`, so the child's state and effect cleanups survive re-renders without the caller threading a store back in. Keying cleanups by `(view_id, hook_index)` this way also means a child's effect at hook index 0 no longer collides with the parent's.
