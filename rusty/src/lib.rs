pub mod core;
pub mod hooks;
pub mod server;
pub mod shared;
pub mod views;
pub mod widgets;

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::core::Runtime;
    pub use crate::hooks::{
        create_context, use_callback, use_context, use_effect, use_effect_with_deps, use_interval,
        use_memo, use_reducer, use_ref, use_state, DynEq, Ref, State,
    };
    pub use crate::server::RustyServer;
    pub use crate::shared::{Align, Color, Density, Icon, Justify, NamedColor, Size};
    pub use crate::views::{BuildContext, Element, View};
    pub use crate::widgets::*;
}

// Re-export the derive macro
pub use rusty_macros::Widget;
