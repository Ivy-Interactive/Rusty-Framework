use rusty::prelude::*;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "memo",
        title: "Memo",
        icon: "brain",
        group: "Concepts",
        order: 2,
        factory: build,
    }
}

fn build(ctx: &mut BuildContext) -> Element {
    let number = use_state(ctx, 10u64);
    let other_state = use_state(ctx, 0i32);

    let num_val = number.get();
    let other_val = other_state.get();

    // Memoized computation - only recalculates when `number` changes
    let fibonacci = use_memo(ctx, &[&num_val as &dyn DynEq], || {
        fn fib(n: u64) -> u64 {
            match n {
                0 => 0,
                1 => 1,
                _ => fib(n - 1) + fib(n - 2),
            }
        }
        fib(num_val)
    });

    let inc_num = number.clone();
    let dec_num = number.clone();
    let inc_other = other_state.clone();

    sample_page(
        "Memo",
        "Demonstrates use_memo for memoizing expensive computations.",
        Layout::vertical()
            .gap(16.0)
            .child(demo_section(
                "Fibonacci Calculator",
                Layout::vertical()
                    .gap(8.0)
                    .child(TextBlock::new(&format!("Input: {num_val}")))
                    .child(TextBlock::new(&format!("Fibonacci({num_val}) = {fibonacci}")))
                    .child(
                        Layout::horizontal()
                            .gap(8.0)
                            .child(Button::new("+1").on_click(move || {
                                inc_num.update(|v| (v + 1).min(30));
                            }))
                            .child(Button::new("-1").on_click(move || {
                                dec_num.update(|v| v.saturating_sub(1));
                            })),
                    )
                    .child(TextBlock::paragraph(
                        "The Fibonacci value is memoized and only recalculates when the input changes.",
                    ))
                    .into(),
            ))
            .child(demo_section(
                "Unrelated State",
                Layout::vertical()
                    .gap(8.0)
                    .child(TextBlock::new(&format!("Other counter: {other_val}")))
                    .child(Button::new("Increment Other").on_click(move || {
                        inc_other.update(|v| v + 1);
                    }))
                    .child(TextBlock::paragraph(
                        "Changing this counter does NOT recalculate the Fibonacci value.",
                    ))
                    .into(),
            ))
            .into(),
    )
}
