//! The IvyML grammar, parsed with `syn` at the token level.
//!
//! Parsing tokens rather than a string is what makes `{expr}` interpolation and
//! span-accurate errors possible: rustc hands a proc macro real tokens, and
//! `ivyml_file!` reaches this same code path by lexing the file text with
//! [`str::parse::<TokenStream>`]. The consequence is that `.ivyml` files must be
//! Rust-lexable, which rules out bare prose in child position — use
//! `content="..."` instead.

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::{braced, Expr, Ident, Lit, Token};

/// A single attribute on an element: `name=literal` or `name={rust_expr}`.
pub struct Attribute {
    pub name: Ident,
    pub value: AttrValue,
}

/// The right-hand side of an attribute.
pub enum AttrValue {
    /// A bare literal, e.g. `gap=16` or `content="Hello"`.
    Literal(Lit),
    /// A braced Rust expression, e.g. `on_click={move || ..}`.
    Expr(Expr),
}

impl AttrValue {
    /// The span to point diagnostics at.
    pub fn span(&self) -> Span {
        match self {
            AttrValue::Literal(lit) => lit.span(),
            AttrValue::Expr(expr) => syn::spanned::Spanned::span(expr),
        }
    }
}

/// Anything that can appear in child position.
pub enum Node {
    Element(ElementNode),
    /// A `{expr}` splice: any expression that converts into an `Element`.
    Expr(Expr),
}

/// One `<Name ...>` element, self-closing or paired.
pub struct ElementNode {
    pub name: Ident,
    pub attrs: Vec<Attribute>,
    pub children: Vec<Node>,
}

impl ElementNode {
    /// Look up an attribute by name.
    pub fn attr(&self, name: &str) -> Option<&Attribute> {
        self.attrs.iter().find(|a| a.name == name)
    }
}

/// The whole macro body: exactly one root element.
pub struct Markup {
    pub root: ElementNode,
}

impl Parse for Attribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = parse_attr_name(input)?;
        input.parse::<Token![=]>()?;

        let value = if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            AttrValue::Expr(content.parse::<Expr>()?)
        } else {
            AttrValue::Literal(input.parse::<Lit>()?)
        };

        Ok(Attribute { name, value })
    }
}

/// Attribute names may collide with Rust keywords (`type`, `for`), so accept any
/// identifier rather than `Ident::parse`, which rejects keywords.
fn parse_attr_name(input: ParseStream) -> syn::Result<Ident> {
    input.call(syn::ext::IdentExt::parse_any)
}

impl Parse for ElementNode {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![<]>()?;
        let name = parse_element_name(input)?;

        let mut attrs = Vec::new();
        while !input.peek(Token![>]) && !input.peek(Token![/]) {
            attrs.push(input.parse::<Attribute>()?);
        }

        // `<Name />` — self-closing, no children.
        if input.peek(Token![/]) {
            input.parse::<Token![/]>()?;
            input.parse::<Token![>]>()?;
            return Ok(ElementNode {
                name,
                attrs,
                children: Vec::new(),
            });
        }

        input.parse::<Token![>]>()?;

        let mut children = Vec::new();
        loop {
            if input.is_empty() {
                return Err(syn::Error::new(
                    name.span(),
                    format!("unclosed element `<{}>`", name),
                ));
            }

            // A `</` starts the closing tag; anything else is another child.
            if input.peek(Token![<]) && input.peek2(Token![/]) {
                break;
            }

            if input.peek(syn::token::Brace) {
                let content;
                braced!(content in input);
                children.push(Node::Expr(content.parse::<Expr>()?));
            } else if input.peek(Token![<]) {
                children.push(Node::Element(input.parse::<ElementNode>()?));
            } else {
                return Err(syn::Error::new(
                    input.span(),
                    "expected a nested element `<Name ...>` or an interpolated \
                     expression `{expr}` here",
                ));
            }
        }

        input.parse::<Token![<]>()?;
        input.parse::<Token![/]>()?;
        let closing = parse_element_name(input)?;
        input.parse::<Token![>]>()?;

        if closing != name {
            return Err(syn::Error::new(
                closing.span(),
                format!("closing tag `</{}>` does not match `<{}>`", closing, name),
            ));
        }

        Ok(ElementNode {
            name,
            attrs,
            children,
        })
    }
}

/// Element names carry an optional `.method` suffix (e.g. `<List.Item>` were it
/// ever needed); today only the bare form is accepted, and rejecting the dotted
/// form here rather than in codegen keeps the error at the tag.
fn parse_element_name(input: ParseStream) -> syn::Result<Ident> {
    let name = input.parse::<Ident>()?;
    if input.peek(Token![.]) {
        return Err(syn::Error::new(
            name.span(),
            "dotted element names are not supported; use a plain tag such as `<List>`",
        ));
    }
    Ok(name)
}

impl Parse for Markup {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Err(syn::Error::new(
                Span::call_site(),
                "expected a single root element, e.g. <Layout>...</Layout>",
            ));
        }

        let root = input.parse::<ElementNode>()?;

        if !input.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "expected a single root element; wrap siblings in a container such as <Layout>",
            ));
        }

        Ok(Markup { root })
    }
}

impl ToTokens for AttrValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            AttrValue::Literal(lit) => lit.to_tokens(tokens),
            AttrValue::Expr(expr) => expr.to_tokens(tokens),
        }
    }
}
