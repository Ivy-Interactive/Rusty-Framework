//! Parses XAML markup into [`rusty`] widget trees at runtime.
//!
//! `rusty-ivyml` compiles markup into builder calls at compile time; this crate
//! does the same job with a `&str` that only exists once the program is running —
//! a document loaded from disk, from a database, or edited while the app is up.
//! The cost of that is the obvious one: a mistake in the markup is a
//! [`XamlError`] rather than a compiler error, so every function here returns a
//! `Result`.
//!
//! ```
//! use rusty_xaml::XamlContext;
//!
//! let ctx = XamlContext::new()
//!     .value("Title", "Dashboard")
//!     .handler("OnIncrement", || println!("clicked"));
//!
//! let element = rusty_xaml::parse_with(
//!     r#"<StackPanel Spacing="8">
//!            <TextBlock Text="{Binding Title}" Variant="Heading1" />
//!            <Button Content="Increment" Click="OnIncrement" />
//!        </StackPanel>"#,
//!     &ctx,
//! )?;
//!
//! # let json = serde_json::to_value(&element).unwrap();
//! # assert_eq!(json["children"][0]["content"], "Dashboard");
//! # Ok::<(), rusty_xaml::XamlError>(())
//! ```
//!
//! # Bindings are a snapshot
//!
//! `{Binding X}` is resolved once, while parsing, from the [`XamlContext`] handed
//! to [`parse_with`]. The returned [`Element`] holds the resolved value, not a
//! live link back to it — there is no data-context machinery here.
//!
//! Re-rendering therefore works exactly as it does for a hand-built tree: call
//! [`parse_with`] *inside* `View::build`, with a context built from the current
//! state. Every rebuild re-parses, and every rebuild sees fresh values. Parsing
//! once into a `static` and expecting a bound label to change will not work.
//!
//! # Widget ids
//!
//! `x:Name` is ignored, along with the rest of the `x:` directives. Rusty assigns
//! widget ids itself during `Element::assign_ids`, keyed by position in the tree,
//! and the event registry is keyed by those ids — so honouring `x:Name` would
//! mean either two id schemes or an id that `assign_ids` immediately overwrites.
//! Naming an element is still useful documentation, so it parses rather than
//! erroring.
//!
//! Events are wired the same way: `Click="OnIncrement"` names a handler that
//! [`XamlContext::handler`] must have registered. The value is a name, never an
//! expression; there is no interpreter in this crate.
//!
//! # What the markup may contain
//!
//! The vocabulary is the WPF-flavoured spelling of Rusty's widgets
//! (`<StackPanel>`, `<Border>`, `<TextBlock Text=".." />`), and each widget's
//! Rusty name is accepted as an alias. Anything outside it — an unknown element,
//! an unknown attribute, an unparseable value, a child on a leaf — is an error
//! rather than a silent omission, because markup that renders half its styling is
//! worse than markup that refuses to render.

mod build;
mod context;
mod error;
mod value;

use std::fs;
use std::path::Path;

use roxmltree::Document;
use rusty::views::Element;

pub use crate::context::{Handler, XamlContext};
pub use crate::error::{Position, XamlError};

/// Parse a XAML document with no bindings and no handlers.
///
/// Equivalent to [`parse_with`] against a default [`XamlContext`]: a document
/// containing `{Binding ..}` or an event attribute fails here, naming what was
/// missing.
pub fn parse(xaml: &str) -> Result<Element, XamlError> {
    parse_with(xaml, &XamlContext::new())
}

/// Parse a XAML document, resolving bindings and handlers against `ctx`.
pub fn parse_with(xaml: &str, ctx: &XamlContext) -> Result<Element, XamlError> {
    let doc = Document::parse(xaml)?;
    build::build_root(&doc, ctx)
}

/// Read a `.xaml` file and parse it.
pub fn parse_file(path: impl AsRef<Path>) -> Result<Element, XamlError> {
    parse_file_with(path, &XamlContext::new())
}

/// Read a `.xaml` file and parse it, resolving against `ctx`.
///
/// The file is read into a `String` first: `roxmltree` borrows from the text for
/// as long as the `Document` lives, while an [`Element`] owns everything it
/// needs, so neither outlives this call.
pub fn parse_file_with(path: impl AsRef<Path>, ctx: &XamlContext) -> Result<Element, XamlError> {
    let path = path.as_ref();
    let xaml = fs::read_to_string(path).map_err(|source| XamlError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    parse_with(&xaml, ctx)
}
