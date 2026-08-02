//! A hook in a loop body allocates a different number of slots per build.
use rusty::prelude::*;

struct App {
    rows: Vec<String>,
}

#[rusty::view]
impl View for App {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let mut layout = Layout::vertical();

        for row in &self.rows {
            let selected = use_state(ctx, false);
            layout = layout.child(TextBlock::new(&format!("{} {}", row, selected.get())));
        }

        layout.into()
    }
}

fn main() {}
