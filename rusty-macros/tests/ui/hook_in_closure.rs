//! A hook inside a closure runs when the closure runs, not in build order.
use rusty::prelude::*;

struct App;

#[rusty::view]
impl View for App {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let make_slot = |c: &mut BuildContext| use_state(c, 0i32);
        let count = make_slot(ctx);

        TextBlock::new(&format!("{}", count.get())).into()
    }
}

fn main() {}
