//! A non-`Option<String>` `id` used to surface as `E0308` plus
//! `E0599: no method named as_deref`.
use rusty_macros::Widget;

#[derive(Widget, Clone, Debug)]
struct Label {
    id: String,
    #[prop]
    text: String,
}

fn main() {}
