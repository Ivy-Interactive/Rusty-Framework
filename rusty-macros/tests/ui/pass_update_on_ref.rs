//! `.update()` on a `use_ref` binding is silent, so it cannot loop. This is the
//! render counter in rusty/examples/hooks_showcase.rs — the third false positive
//! a name-only version of the rule produced on a green tree.
use rusty::prelude::*;

struct App;

#[rusty::view]
impl View for App {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let render_count = use_ref(ctx, 0i32);
        render_count.update(|n| n + 1);

        let alias = render_count.clone();
        alias.set(alias.get());

        TextBlock::new(&format!("builds: {}", render_count.get())).into()
    }
}

fn main() {}
