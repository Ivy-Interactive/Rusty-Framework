pub mod core;
pub mod hooks;
pub mod server;
pub mod shared;
pub mod views;
pub mod widgets;

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::core::Runtime;
    pub use crate::hooks::{use_callback, use_effect, use_memo, use_state, State};
    pub use crate::server::RustyServer;
    pub use crate::shared::{Align, Color, Density, Icon, Justify, NamedColor, Size};
    pub use crate::views::{BuildContext, Element, View};
    pub use crate::widgets::*;
}

// Re-export the derive macro
pub use rusty_macros::Widget;
