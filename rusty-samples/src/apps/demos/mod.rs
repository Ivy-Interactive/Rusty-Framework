mod counter_app;
mod hello_app;
mod todo_app;

use crate::apps::AppEntry;

pub fn register() -> Vec<AppEntry> {
    vec![hello_app::entry(), counter_app::entry(), todo_app::entry()]
}
