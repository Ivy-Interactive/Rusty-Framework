//! Compiles `.ivyml` declarative markup into Rusty widget trees.
//!
//! Two function-like proc macros, both re-exported from `rusty`:
//!
//! - [`ivyml!`] — inline markup.
//! - [`ivyml_file!`] — an external `.ivyml` file, resolved relative to
//!   `CARGO_MANIFEST_DIR`.
//!
//! Both lower to the ordinary builder chains in `rusty::widgets`, so there is no
//! new runtime, no interpreter and no wire-format change. A malformed tag is a
//! `rustc` error with a span, not a panic in production.
//!
//! ```ignore
//! use rusty::prelude::*;
//! use rusty::ivyml;
//!
//! impl View for Counter {
//!     fn build(&self, ctx: &mut BuildContext) -> Element {
//!         let count = use_state(ctx, || 0i32);
//!         ivyml! {
//!             <Layout direction="vertical" gap=16 padding=24>
//!                 <TextBlock content="Counter" variant="heading1" />
//!                 <TextBlock content={format!("count = {}", count.get())} />
//!                 <Button title="Inc" on_click={move || count.update(|v| v + 1)} />
//!             </Layout>
//!         }
//!     }
//! }
//! ```

mod ast;
mod codegen;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

/// Compile inline IvyML markup into a `rusty::views::Element`.
///
/// One root element per invocation. Attributes are `name=literal` or
/// `name={rust_expr}`; children are nested elements or `{expr}` splices. Both
/// `<Card />` and `<Card></Card>` parse.
#[proc_macro]
pub fn ivyml(input: TokenStream) -> TokenStream {
    let markup = parse_macro_input!(input as ast::Markup);
    match codegen::root_tokens(&markup.root) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Compile an external `.ivyml` file into a `rusty::views::Element`.
///
/// The path is resolved against `CARGO_MANIFEST_DIR`, so it is relative to the
/// crate root rather than to the source file.
///
/// ```ignore
/// ivyml_file!("src/views/dashboard.ivyml")
/// ```
#[proc_macro]
pub fn ivyml_file(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as syn::LitStr);
    let rel_path = lit.value();

    let manifest_dir = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => dir,
        Err(_) => {
            return syn::Error::new(
                lit.span(),
                "CARGO_MANIFEST_DIR is not set; ivyml_file! must be expanded by cargo",
            )
            .to_compile_error()
            .into()
        }
    };

    let full_path = std::path::Path::new(&manifest_dir).join(&rel_path);
    let text = match std::fs::read_to_string(&full_path) {
        Ok(text) => text,
        Err(err) => {
            let msg = format!("cannot read `{}`: {}", full_path.display(), err);
            return syn::Error::new(lit.span(), msg).to_compile_error().into();
        }
    };

    // Lexing the file text reaches the same parser as the inline form, which is
    // what gives `.ivyml` files `{expr}` interpolation. The cost is that they
    // must be Rust-lexable: no bare prose in child position.
    let stream: proc_macro2::TokenStream = match text.parse() {
        Ok(stream) => stream,
        Err(err) => {
            let msg = format!("`{}` is not lexable as Rust tokens: {}", rel_path, err);
            return syn::Error::new(lit.span(), msg).to_compile_error().into();
        }
    };

    let markup = match syn::parse2::<ast::Markup>(stream) {
        Ok(markup) => markup,
        Err(err) => return err.to_compile_error().into(),
    };

    let tokens = match codegen::root_tokens(&markup.root) {
        Ok(tokens) => tokens,
        Err(err) => return err.to_compile_error().into(),
    };

    let path_str = full_path.to_string_lossy().to_string();

    quote! {{
        // Rebuild when the markup changes, not just when the .rs file does. A
        // proc macro that reads a file has no dependency edge to it, and
        // `cargo:rerun-if-changed` is a build-script mechanism unavailable here,
        // so without this line cargo serves a stale expansion: editing the
        // .ivyml and rebuilding prints the old text. Do not remove.
        const _: &str = include_str!(#path_str);
        #tokens
    }}
    .into()
}
