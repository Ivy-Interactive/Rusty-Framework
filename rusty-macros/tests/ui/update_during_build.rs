//! Same rule through a clone alias and through `update` instead of `set`.
use rusty::prelude::*;

struct App;

#[rusty::view]
impl View for App {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let count = use_state(ctx, 0i32);
        let alias = count.clone();
        alias.update(|n| n + 1);

        TextBlock::new(&format!("{}", count.get())).into()
    }
}

fn main() {}
