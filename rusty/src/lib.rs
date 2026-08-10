pub mod core;
pub mod hooks;
pub mod server;
pub mod shared;
pub mod views;
pub mod widgets;

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::core::{
        AppDescriptor, AppFactory, AppIds, AppRegistry, QueryError, QueryOptions, QueryScope,
        Runtime, ServiceRegistry, Signal, SignalScope, ViewTree,
    };
    pub use crate::hooks::{
        create_context, try_use_service, use_alert, use_callback, use_context, use_download,
        use_download_bytes, use_download_stream, use_effect, use_effect_with_deps, use_form,
        use_interval, use_memo, use_mutation, use_query, use_reducer, use_ref, use_service,
        use_signal, use_state, use_stream, use_stream_text, use_trigger, use_trigger_unit,
        use_upload, use_upload_to, DynEq, QueryMutator, QueryResult, Ref, ShowAlert, State,
        StreamOptions, StreamStatus, UploadStatus,
    };
    pub use crate::server::upload::{
        UploadConstraints, UploadError, UploadEvent, UploadedFile, DEFAULT_MAX_UPLOAD_BYTES,
    };
    pub use crate::server::{RustyServer, DEFAULT_BIND_ADDRESS};
    pub use crate::shared::{Align, Color, Density, Icon, Justify, NamedColor, Size};
    pub use crate::views::{
        AlertButtonSet, AlertResult, BuildContext, Element, FieldRender, FormBuilder, ModelSetter,
        SubmitHandler, Validator, View,
    };
    pub use crate::widgets::*;
    /// The crate itself, so `rusty_filter::lexer` and the other modules are
    /// reachable without a second dependency line in `Cargo.toml`.
    pub use rusty_filter;
    pub use rusty_filter::{
        canonical_key, count_matches, evaluate, parse_query, parse_query_unchecked,
        retain_matching, to_query_string, validate_filter_group, ColumnDef, ColumnType, Condition,
        ErrorSeverity, Filter, FilterFunction, FilterGroup, LogicalOp, ParseError, ParseResult,
    };
}

// Re-export the derive macro
pub use rusty_macros::Widget;

/// Re-export of the hook-invariant attribute macro, so it reads as
/// `#[rusty::view]` on an `impl View for X` block.
///
/// It checks `fn build` for conditional hook calls and for `State::set` /
/// `State::update` during build, and never alters the code it annotates. See
/// [`rusty_macros::view`] for the rules and the `allow(..)` escape hatch.
pub use rusty_macros::view;

// Re-export the markup macros. Deliberately not in `prelude`: a glob-imported
// `ivyml!` reads as a locally defined macro, and the prelude exports only types.
pub use rusty_ivyml::{ivyml, ivyml_file};
