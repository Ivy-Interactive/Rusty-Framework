//! `.set()` inside `tokio::spawn(async move { .. })` is deferred, so it does not
//! rebuild the view currently building. This is the shape in
//! rusty/src/core/runtime.rs, and a name-only version of the rule reported it as
//! a false positive.
use rusty::prelude::*;

struct App;

#[rusty::view]
impl View for App {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let status = use_state(ctx, String::new());
        let setter = status.clone();

        tokio::spawn(async move {
            setter.set("from task".to_string());
        });

        TextBlock::new(&status.get()).into()
    }
}

fn main() {}
