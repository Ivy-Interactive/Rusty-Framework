use rusty::prelude::*;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "effect",
        title: "Effect",
        icon: "zap",
        group: "Concepts",
        order: 1,
        factory: build,
    }
}

fn build(ctx: &mut BuildContext) -> Element {
    let mount_message = use_state(ctx, "Not mounted yet".to_string());
    let click_count = use_state(ctx, 0u32);
    let effect_log = use_state(ctx, "Waiting for effect...".to_string());

    // Mount-only effect
    let mount_msg = mount_message.clone();
    use_effect(ctx, move || {
        mount_msg.set("Component mounted!".to_string());
        None
    });

    // Effect with dependencies
    let count_val = click_count.get();
    let log_state = effect_log.clone();
    use_effect_with_deps(ctx, &[&count_val as &dyn DynEq], move |_| {
        log_state.set(format!("Effect ran! Click count is now: {count_val}"));
        None
    });

    let inc = click_count.clone();

    sample_page(
        "Effect",
        "Demonstrates use_effect and use_effect_with_deps for side effects.",
        Layout::vertical()
            .gap(16.0)
            .child(demo_section(
                "Mount Effect (use_effect)",
                Layout::vertical()
                    .gap(8.0)
                    .child(TextBlock::new(&mount_message.get()))
                    .child(TextBlock::paragraph(
                        "This effect runs once when the component mounts.",
                    ))
                    .into(),
            ))
            .child(demo_section(
                "Dependency Effect (use_effect_with_deps)",
                Layout::vertical()
                    .gap(8.0)
                    .child(TextBlock::new(&format!("Clicks: {count_val}")))
                    .child(TextBlock::new(&effect_log.get()))
                    .child(Button::new("Click Me").on_click(move || {
                        inc.update(|v| v + 1);
                    }))
                    .child(TextBlock::paragraph(
                        "The effect re-runs each time the click count changes.",
                    ))
                    .into(),
            ))
            .into(),
    )
}
