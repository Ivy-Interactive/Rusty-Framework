//! `#[prop]` plus `#[event]` on one field used to surface as
//! `E0277: Arc<dyn Fn(..)>: Serialize is not satisfied`, which says nothing
//! about the two attributes.
use rusty_macros::Widget;
use std::sync::Arc;

#[derive(Widget, Clone)]
struct Control {
    id: Option<String>,
    #[prop]
    #[event]
    on_click: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for Control {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Control")
    }
}

fn main() {}
