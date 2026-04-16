mod context_app;
mod effect_app;
mod interval_app;
mod memo_app;
mod reducer_app;
mod state_app;

use crate::apps::AppEntry;

pub fn register() -> Vec<AppEntry> {
    vec![
        state_app::entry(),
        effect_app::entry(),
        memo_app::entry(),
        reducer_app::entry(),
        context_app::entry(),
        interval_app::entry(),
    ]
}
