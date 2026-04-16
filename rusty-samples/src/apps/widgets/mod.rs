mod badge_app;
mod button_app;
mod card_app;
mod dialog_app;
mod input_app;
mod layout_app;
mod progress_app;
mod table_app;
mod tooltip_app;

use crate::apps::AppEntry;

pub fn register() -> Vec<AppEntry> {
    vec![
        button_app::entry(),
        card_app::entry(),
        table_app::entry(),
        input_app::entry(),
        dialog_app::entry(),
        badge_app::entry(),
        progress_app::entry(),
        tooltip_app::entry(),
        layout_app::entry(),
    ]
}
