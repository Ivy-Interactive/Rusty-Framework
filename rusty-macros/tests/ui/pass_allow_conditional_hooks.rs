//! `allow(conditional_hooks)` suppresses rule A for the whole impl block. The
//! body here is the exact one `hook_in_if.rs` rejects.
use rusty::prelude::*;

struct App {
    flag: bool,
}

#[rusty::view(allow(conditional_hooks))]
impl View for App {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let count = use_state(ctx, 0i32);

        if self.flag {
            let extra = use_state(ctx, 0i32);
            let _ = extra.get();
        }

        TextBlock::new(&format!("{}", count.get())).into()
    }
}

fn main() {}
