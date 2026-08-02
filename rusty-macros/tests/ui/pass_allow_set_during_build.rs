//! `allow(set_during_build)` suppresses rule B for the whole impl block. The
//! body here is the exact one `set_during_build.rs` rejects.
use rusty::prelude::*;

struct App;

#[rusty::view(allow(set_during_build))]
impl View for App {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let count = use_state(ctx, 0i32);
        count.set(1);

        TextBlock::new(&format!("{}", count.get())).into()
    }
}

fn main() {}
