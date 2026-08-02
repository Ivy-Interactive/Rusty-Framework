//! `#[event]` on a non-`Option` field used to surface as
//! `E0599: no method named is_some found for Arc<dyn Fn()>`.
use rusty_macros::Widget;
use std::sync::Arc;

#[derive(Widget, Clone)]
struct Control {
    id: Option<String>,
    #[prop]
    label: String,
    #[event]
    on_click: Arc<dyn Fn() + Send + Sync>,
}

impl std::fmt::Debug for Control {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Control")
    }
}

fn main() {}
