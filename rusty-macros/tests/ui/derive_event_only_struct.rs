//! A struct with events but no props and no `id` serializes to just
//! `{"type": ".."}` plus `has<Event>` flags. This compiled clean before.
use rusty_macros::Widget;
use std::sync::Arc;

#[derive(Widget, Clone)]
struct Trigger {
    #[event]
    on_click: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for Trigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Trigger")
    }
}

fn main() {}
