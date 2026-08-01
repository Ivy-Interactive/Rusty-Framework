//! A hook called inside an `if` branch shifts every later hook's slot.
use rusty::prelude::*;

struct App {
    flag: bool,
}

#[rusty::view]
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
