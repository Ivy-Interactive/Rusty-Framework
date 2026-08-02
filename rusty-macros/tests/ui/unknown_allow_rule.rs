//! A typo in an `allow(..)` must be an error, not a silently disabled lint.
use rusty::prelude::*;

struct App;

#[rusty::view(allow(conditional_hook))]
impl View for App {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        TextBlock::new("hello").into()
    }
}

fn main() {}
