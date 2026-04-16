use std::time::Duration;

use rusty::prelude::*;
use rusty::widgets::button::ButtonVariant;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "interval",
        title: "Interval",
        icon: "timer",
        group: "Concepts",
        order: 5,
        factory: build,
    }
}

fn build(ctx: &mut BuildContext) -> Element {
    let ticks = use_state(ctx, 0u32);
    let running = use_state(ctx, true);

    let tick_state = ticks.clone();
    let is_running = running.get();

    let duration = if is_running {
        Some(Duration::from_secs(1))
    } else {
        None
    };

    use_interval(ctx, duration, move || {
        tick_state.update(|v| v + 1);
    });

    let tick_val = ticks.get();
    let toggle = running.clone();
    let reset_ticks = ticks.clone();

    let hours = tick_val / 3600;
    let minutes = (tick_val % 3600) / 60;
    let seconds = tick_val % 60;
    let time_display = format!("{hours:02}:{minutes:02}:{seconds:02}");

    sample_page(
        "Interval",
        "Demonstrates use_interval for recurring timers and clocks.",
        Layout::vertical()
            .gap(16.0)
            .child(demo_section(
                "Timer",
                Layout::vertical()
                    .gap(12.0)
                    .child(TextBlock::h1(&time_display))
                    .child(TextBlock::new(&format!("Total ticks: {tick_val}")))
                    .child(
                        Layout::horizontal()
                            .gap(8.0)
                            .child(
                                Button::new(if is_running { "Pause" } else { "Resume" }).on_click(
                                    move || {
                                        toggle.update(|v| !v);
                                    },
                                ),
                            )
                            .child(Button::new("Reset").variant(ButtonVariant::Ghost).on_click(
                                move || {
                                    reset_ticks.set(0);
                                },
                            )),
                    )
                    .child(TextBlock::paragraph(
                        "Pass None as the duration to pause the interval. \
                         Pass Some(duration) to resume.",
                    ))
                    .into(),
            ))
            .into(),
    )
}
