//! `.set()` inside an event handler is the correct shape, used by every example
//! in rusty/examples. It must never be flagged.
use rusty::prelude::*;

struct App;

#[rusty::view]
impl View for App {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let count = use_state(ctx, 0i32);
        let count_inc = count.clone();
        let count_reset = count.clone();

        Layout::vertical()
            .child(TextBlock::new(&format!("{}", count.get())))
            .child(Button::new("Increment").on_click(move || {
                count_inc.update(|n| n + 1);
            }))
            .child(Button::new("Reset").on_click(move || {
                count_reset.set(0);
            }))
            .into()
    }
}

fn main() {}
