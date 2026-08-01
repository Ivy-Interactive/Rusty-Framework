//! A hook in an `if` *condition* runs on every build, so its slot is stable.
//! This is the shape in rusty/src/hooks/use_trigger.rs.
use rusty::prelude::*;

struct App;

#[rusty::view]
impl View for App {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        if use_state(ctx, false).get() {
            return TextBlock::new("open").into();
        }

        let value = use_ref(ctx, 0i32);
        TextBlock::new(&format!("{}", value.get())).into()
    }
}

fn main() {}
