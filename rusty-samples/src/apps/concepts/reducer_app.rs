use rusty::prelude::*;
use rusty::widgets::button::ButtonVariant;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "reducer",
        title: "Reducer",
        icon: "workflow",
        group: "Concepts",
        order: 3,
        factory: build,
    }
}

#[derive(Clone)]
enum CounterAction {
    Increment,
    Decrement,
    Add(i32),
    Reset,
}

fn counter_reducer(state: &i32, action: CounterAction) -> i32 {
    match action {
        CounterAction::Increment => state + 1,
        CounterAction::Decrement => state - 1,
        CounterAction::Add(n) => state + n,
        CounterAction::Reset => 0,
    }
}

fn build(ctx: &mut BuildContext) -> Element {
    let (state, dispatch) = use_reducer(ctx, counter_reducer, 0);

    let val = state.get();
    let d_inc = dispatch.clone();
    let d_dec = dispatch.clone();
    let d_add5 = dispatch.clone();
    let d_add10 = dispatch.clone();
    let d_reset = dispatch.clone();

    sample_page(
        "Reducer",
        "Demonstrates use_reducer for complex state logic with actions.",
        demo_section(
            "Counter with Reducer",
            Layout::vertical()
                .gap(12.0)
                .child(TextBlock::h2(&format!("Value: {val}")))
                .child(
                    Layout::horizontal()
                        .gap(8.0)
                        .child(Button::new("+1").on_click(move || {
                            d_inc(CounterAction::Increment);
                        }))
                        .child(Button::new("-1").on_click(move || {
                            d_dec(CounterAction::Decrement);
                        }))
                        .child(Button::new("+5").on_click(move || {
                            d_add5(CounterAction::Add(5));
                        }))
                        .child(Button::new("+10").on_click(move || {
                            d_add10(CounterAction::Add(10));
                        }))
                        .child(
                            Button::new("Reset")
                                .variant(ButtonVariant::Ghost)
                                .on_click(move || {
                                    d_reset(CounterAction::Reset);
                                }),
                        ),
                )
                .child(TextBlock::paragraph(
                    "Actions like Increment, Decrement, Add(n), and Reset are dispatched to a reducer function.",
                ))
                .into(),
        ),
    )
}
