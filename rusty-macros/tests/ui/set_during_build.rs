//! `State::set` during build requests a rebuild of the view being built.
use rusty::prelude::*;

struct App;

#[rusty::view]
impl View for App {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let count = use_state(ctx, 0i32);
        count.set(1);

        TextBlock::new(&format!("{}", count.get())).into()
    }
}

fn main() {}
