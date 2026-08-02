//! Both rules named in one `allow(..)` group.
use rusty::prelude::*;

struct App {
    flag: bool,
}

#[rusty::view(allow(conditional_hooks, set_during_build))]
impl View for App {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let count = use_state(ctx, 0i32);
        count.set(1);

        if self.flag {
            let extra = use_state(ctx, 0i32);
            let _ = extra.get();
        }

        TextBlock::new(&format!("{}", count.get())).into()
    }
}

fn main() {}
