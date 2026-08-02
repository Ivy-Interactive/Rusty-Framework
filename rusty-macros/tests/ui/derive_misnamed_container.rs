//! A `Vec<Element>` field not named `children` gets no `children_mut`, so
//! `Element::assign_ids` never descends into it. This compiled clean before the
//! diagnostic existed.
use rusty::views::Element;
use rusty_macros::Widget;

#[derive(Widget, Clone, Debug)]
struct Panel {
    id: Option<String>,
    #[prop]
    title: String,
    items: Vec<Element>,
}

fn main() {}
