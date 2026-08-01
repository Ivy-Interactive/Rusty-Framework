//! Native desktop app shell for Rusty-Framework applications.
//!
//! A Rusty app normally runs as `rusty-server` plus a browser. This crate turns one into a
//! double-clickable executable: it starts the `rusty` server in-process on an ephemeral
//! loopback port, serves the renderer from bytes compiled into the binary, opens a native
//! window against it, and drives a native menu bar.
//!
//! The crate is split by what can be tested. [`assets`], [`menu`] and [`desktop_router`]
//! have no GUI dependencies and are covered by unit and integration tests. [`shell`] sits
//! behind the default-on `shell` feature and is not covered: `tao` refuses to build an
//! event loop off the main thread (so `cargo test` cannot construct one) and
//! `muda::MenuEvent::send` is `pub(crate)` (so a click cannot be synthesized). CI builds
//! `--no-default-features` because `ubuntu-latest` has no `libwebkit2gtk-4.1-dev`.

use axum::response::Html;
use axum::routing::get;
use axum::Router;
use rusty::prelude::RustyServer;

pub mod assets;
pub mod menu;
#[cfg(feature = "shell")]
pub mod shell;

/// Build the router the desktop shell serves: the framework's own routes plus `GET /`
/// returning the embedded renderer.
///
/// `RustyServer::with_static_dir` installs a `ServeDir` fallback, which needs files on
/// disk. Layering a `/` route onto [`RustyServer::router`] serves the same client from the
/// executable and leaves `/ws`, `/health` and the download route untouched.
pub fn desktop_router(server: RustyServer) -> Router {
    server
        .router()
        .route("/", get(|| async { Html(assets::index_html()) }))
}
