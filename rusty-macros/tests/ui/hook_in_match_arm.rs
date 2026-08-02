//! A hook in a `match` arm runs on some builds and not others.
use rusty::prelude::*;

struct App {
    mode: u8,
}

#[rusty::view]
impl View for App {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        match self.mode {
            0 => {
                let value = use_ref(ctx, 0i32);
                TextBlock::new(&format!("{}", value.get())).into()
            }
            _ => Element::Empty,
        }
    }
}

fn main() {}
