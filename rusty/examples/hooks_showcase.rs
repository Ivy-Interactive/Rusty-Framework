//! Every hook in one view, each with a visible readout.
//!
//! This doubles as the executable reference for the hook signatures: if a doc
//! snippet disagrees with this file, the doc snippet is wrong.
//!
//! Run with `cargo run --example hooks_showcase` (set `PORT` to override 3000).

use rusty::prelude::*;
use std::time::Duration;

struct HooksApp;

#[rusty::view]
impl View for HooksApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        // use_state takes the initial *value*, not a closure.
        let count = use_state(ctx, 0i32);
        let count_val = count.get();
        let count_inc = count.clone();

        // use_ref is use_state that does not trigger a rebuild on mutation.
        let render_count = use_ref(ctx, 0i32);
        render_count.update(|n| n + 1);
        let renders = render_count.get();

        // use_memo recomputes only when its deps change.
        let doubled = use_memo(ctx, &[&count_val], || count_val * 2);

        // The use_effect closure returns Option<Box<dyn FnOnce() + Send + Sync>>
        // — an optional cleanup function, not (). It runs once on mount.
        use_effect(ctx, || {
            println!("mounted");
            Some(Box::new(|| println!("unmounted")) as Box<dyn FnOnce() + Send + Sync>)
        });

        // use_effect_with_deps takes deps *second* and the callback third. The
        // callback receives the deps and returns the same optional cleanup.
        use_effect_with_deps(ctx, &[&count_val], move |deps| {
            if let Some(value) = deps[0].as_any().downcast_ref::<i32>() {
                println!("count changed to {}", value);
            }
            None
        });

        // use_interval takes Option<Duration>; None pauses it.
        let ticks = use_state(ctx, 0u32);
        let ticks_val = ticks.get();
        let ticks_bump = ticks.clone();
        use_interval(ctx, Some(Duration::from_secs(1)), move || {
            ticks_bump.update(|n| n + 1);
        });

        Layout::vertical()
            .padding(24.0)
            .gap(16.0)
            .child(TextBlock::h1("Hooks Showcase"))
            .child(
                Card::new()
                    .title("use_state")
                    .child(TextBlock::paragraph(&format!("Count: {}", count_val)))
                    .child(Button::new("Increment").on_click(move || {
                        count_inc.update(|n| n + 1);
                    })),
            )
            .child(
                Card::new()
                    .title("use_ref")
                    .subtitle("Mutating a ref does not trigger a rebuild")
                    .child(TextBlock::paragraph(&format!("Builds so far: {}", renders))),
            )
            .child(
                Card::new()
                    .title("use_memo")
                    .child(TextBlock::paragraph(&format!("Count doubled: {}", doubled))),
            )
            .child(
                Card::new()
                    .title("use_effect / use_effect_with_deps")
                    .subtitle("Both log to stdout — watch the console"),
            )
            .child(
                Card::new()
                    .title("use_interval")
                    .child(TextBlock::paragraph(&format!("Ticks: {}", ticks_val))),
            )
            .into()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    RustyServer::new(port, || HooksApp).serve().await
}
