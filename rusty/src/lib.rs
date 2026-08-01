pub mod core;
pub mod hooks;
pub mod server;
pub mod shared;
pub mod views;
pub mod widgets;

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::core::{
        QueryError, QueryOptions, QueryScope, Runtime, ServiceRegistry, Signal, SignalScope,
        ViewTree,
    };
    pub use crate::hooks::{
        create_context, use_alert, use_callback, use_context, use_download, use_download_bytes,
        use_effect, use_effect_with_deps, use_interval, use_memo, use_mutation, use_query,
        use_reducer, use_ref, use_service, use_signal, use_state, use_trigger, use_trigger_unit,
        DynEq, QueryMutator, QueryResult, Ref, ShowAlert, State,
    };
    pub use crate::server::RustyServer;
    pub use crate::shared::{Align, Color, Density, Icon, Justify, NamedColor, Size};
    pub use crate::views::{AlertButtonSet, AlertResult, BuildContext, Element, View};
    pub use crate::widgets::*;
}

// Re-export the derive macro
pub use rusty_macros::Widget;
