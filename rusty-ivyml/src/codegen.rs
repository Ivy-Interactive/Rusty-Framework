//! Lowering from the [`crate::ast`] tree to the builder chains that already
//! exist in `rusty::widgets`.
//!
//! There is no runtime here and no new wire format: `<Layout direction="vertical">`
//! becomes `Layout::vertical()`, and every attribute becomes a builder call on it.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{Ident, Lit};

use crate::ast::{AttrValue, Attribute, ElementNode, Node};

/// How one element name maps onto Rusty's builders.
///
/// The mapping cannot be derived from the tag name: the constructors are not
/// uniform (`Layout::vertical()` vs `TextBlock::new(content)` vs `Card::new()`),
/// and `List` attaches children with `.item()` because it stores `items`, not
/// `children`, and has no `.child` method at all.
struct Shape {
    /// The constructor call, already complete.
    ctor: TokenStream,
    /// The builder method children attach through, or `None` for a leaf.
    child_method: Option<Ident>,
    /// Attributes the constructor consumed, which must not also be emitted as
    /// builder calls.
    consumed_by_ctor: Vec<&'static str>,
}

/// Lower one element to its builder chain.
pub fn element_tokens(el: &ElementNode) -> syn::Result<TokenStream> {
    let shape = shape_for(el)?;

    let mut chain = shape.ctor;

    for attr in &el.attrs {
        let name = attr.name.to_string();
        if shape.consumed_by_ctor.contains(&name.as_str()) {
            continue;
        }
        let method = format_ident!("{}", name.replace('-', "_"), span = attr.name.span());
        let arg = coerce(&el.name, &name, &attr.value)?;
        chain = quote! { #chain.#method(#arg) };
    }

    if el.children.is_empty() {
        return Ok(chain);
    }

    let Some(child_method) = shape.child_method else {
        return Err(syn::Error::new(
            el.name.span(),
            format!("`<{}>` does not accept children", el.name),
        ));
    };

    for child in &el.children {
        let child_tokens = match child {
            Node::Element(nested) => element_tokens(nested)?,
            Node::Expr(expr) => quote! { #expr },
        };
        chain = quote! { #chain.#child_method(#child_tokens) };
    }

    Ok(chain)
}

/// Resolve the element name to its [`Shape`], validating required attributes.
fn shape_for(el: &ElementNode) -> syn::Result<Shape> {
    let name = el.name.to_string();
    let span = el.name.span();
    let ty = &el.name;

    let shape = match name.as_str() {
        "Layout" => {
            let direction = el.attr("direction");
            let ctor = match direction {
                None => quote! { ::rusty::widgets::Layout::vertical() },
                Some(attr) => {
                    let value = string_literal(attr, "direction")?;
                    match value.as_str() {
                        "vertical" => quote! { ::rusty::widgets::Layout::vertical() },
                        "horizontal" => quote! { ::rusty::widgets::Layout::horizontal() },
                        "grid" => {
                            let columns = el.attr("columns").ok_or_else(|| {
                                syn::Error::new(
                                    span,
                                    "`<Layout direction=\"grid\">` requires `columns`",
                                )
                            })?;
                            let columns = coerce(ty, "columns", &columns.value)?;
                            quote! { ::rusty::widgets::Layout::grid(#columns) }
                        }
                        other => {
                            let msg = format!(
                                "unknown direction `{}`; expected vertical, horizontal or grid",
                                other
                            );
                            return Err(syn::Error::new(attr.value.span(), msg));
                        }
                    }
                }
            };
            Shape {
                ctor,
                child_method: Some(child_ident(span)),
                consumed_by_ctor: vec!["direction", "columns"],
            }
        }
        "TextBlock" => Shape {
            ctor: required_str_ctor(el, "content")?,
            child_method: None,
            consumed_by_ctor: vec!["content"],
        },
        "Button" => Shape {
            ctor: required_str_ctor(el, "title")?,
            child_method: None,
            consumed_by_ctor: vec!["title"],
        },
        "ListItem" => Shape {
            ctor: required_str_ctor(el, "title")?,
            child_method: None,
            consumed_by_ctor: vec!["title"],
        },
        "Badge" => Shape {
            ctor: required_str_ctor(el, "label")?,
            child_method: None,
            consumed_by_ctor: vec!["label"],
        },
        "Card" => Shape {
            ctor: quote! { ::rusty::widgets::Card::new() },
            child_method: Some(child_ident(span)),
            consumed_by_ctor: Vec::new(),
        },
        "Container" => Shape {
            ctor: quote! { ::rusty::widgets::Container::new() },
            child_method: Some(child_ident(span)),
            consumed_by_ctor: Vec::new(),
        },
        "List" => Shape {
            // `List` stores `items`, which is why `child_method` is per-element.
            ctor: quote! { ::rusty::widgets::List::new() },
            child_method: Some(Ident::new("item", span)),
            consumed_by_ctor: Vec::new(),
        },
        "TextInput" => Shape {
            ctor: quote! { ::rusty::widgets::TextInput::new() },
            child_method: None,
            consumed_by_ctor: Vec::new(),
        },
        "Spacer" => Shape {
            ctor: quote! { ::rusty::widgets::Spacer::new() },
            child_method: None,
            consumed_by_ctor: Vec::new(),
        },
        other => {
            let msg = format!("unknown IvyML element `<{}>`", other);
            return Err(syn::Error::new(span, msg));
        }
    };

    Ok(shape)
}

fn child_ident(span: Span) -> Ident {
    Ident::new("child", span)
}

/// Build a `Type::new(arg)` constructor from a required `&str` attribute.
fn required_str_ctor(el: &ElementNode, attr_name: &str) -> syn::Result<TokenStream> {
    let attr = el.attr(attr_name).ok_or_else(|| {
        let msg = format!("`<{}>` requires `{}`", el.name, attr_name);
        syn::Error::new(el.name.span(), msg)
    })?;
    let ty = &el.name;
    let arg = str_arg(&attr.value);
    Ok(quote! { ::rusty::widgets::#ty::new(#arg) })
}

/// Coerce an attribute value to the argument the builder slot expects.
///
/// The `on_` prefix is checked **first**: an event handler must be passed by
/// value, and falling through to the `&str` default below borrows the closure
/// into a temporary that cannot satisfy the `'static` bound (`E0716`).
fn coerce(element: &Ident, attr: &str, value: &AttrValue) -> syn::Result<TokenStream> {
    if attr.starts_with("on_") {
        return match value {
            AttrValue::Expr(expr) => Ok(quote! { #expr }),
            AttrValue::Literal(lit) => Err(syn::Error::new(
                lit.span(),
                "an event handler must be an interpolated closure, e.g. on_click={|| ..}",
            )),
        };
    }

    match attr {
        "gap" | "padding" | "value" | "min" | "max" | "step" if !is_text_value(element, attr) => {
            float_arg(value)
        }
        "columns" => usize_arg(value),
        "disabled" | "loading" | "wrap" | "border" | "rounded" | "read_only" => bool_arg(value),
        "width" | "height" => size_arg(value),
        "align" => enum_arg(value, "Align", &ALIGN_VARIANTS),
        "justify" => enum_arg(value, "Justify", &JUSTIFY_VARIANTS),
        "variant" => variant_arg(element, value),
        _ => Ok(str_arg(value)),
    }
}

/// `<TextInput value="x">` takes a `&str`, unlike `<Progress value=0.5>`.
fn is_text_value(element: &Ident, attr: &str) -> bool {
    attr == "value" && element == "TextInput"
}

/// Emit `&(expr)` for `&str` slots so an interpolated `String`, `&String` or
/// `format!(..)` all coerce. Requiring `.as_str()` inside markup would put type
/// noise on every line that interpolates.
fn str_arg(value: &AttrValue) -> TokenStream {
    match value {
        AttrValue::Literal(lit) => quote! { #lit },
        AttrValue::Expr(expr) => quote! { &(#expr) },
    }
}

fn float_arg(value: &AttrValue) -> syn::Result<TokenStream> {
    match value {
        AttrValue::Literal(Lit::Int(int)) => {
            let v = int.base10_parse::<i64>()? as f64;
            Ok(quote! { #v })
        }
        AttrValue::Literal(Lit::Float(float)) => {
            let v = float.base10_parse::<f64>()?;
            Ok(quote! { #v })
        }
        AttrValue::Literal(other) => {
            Err(syn::Error::new(other.span(), "expected a f64 literal here"))
        }
        AttrValue::Expr(expr) => Ok(quote! { #expr }),
    }
}

fn usize_arg(value: &AttrValue) -> syn::Result<TokenStream> {
    match value {
        AttrValue::Literal(Lit::Int(int)) => {
            let v = int.base10_parse::<usize>()?;
            Ok(quote! { #v })
        }
        AttrValue::Literal(other) => Err(syn::Error::new(
            other.span(),
            "expected a usize literal here",
        )),
        AttrValue::Expr(expr) => Ok(quote! { #expr }),
    }
}

fn bool_arg(value: &AttrValue) -> syn::Result<TokenStream> {
    match value {
        AttrValue::Literal(Lit::Bool(b)) => Ok(quote! { #b }),
        AttrValue::Literal(other) => Err(syn::Error::new(
            other.span(),
            "expected `true` or `false` here",
        )),
        AttrValue::Expr(expr) => Ok(quote! { #expr }),
    }
}

/// `width="240px"` becomes `Size::Px(240.0)`, not a bare number.
///
/// This is correctness, not convenience: `Size` derives `#[serde(untagged)]`, so
/// `Px(240.0)` and `Percent(240.0)` both serialize to `240.0` and the widgets
/// emit `Size::to_css()` by hand. Guessing the wrong variant silently changes
/// the CSS that reaches the client.
fn size_arg(value: &AttrValue) -> syn::Result<TokenStream> {
    let lit = match value {
        AttrValue::Expr(expr) => return Ok(quote! { #expr }),
        AttrValue::Literal(lit) => lit,
    };

    let text = match lit {
        Lit::Str(s) => s.value(),
        // `width=240` is unambiguous: pixels.
        Lit::Int(int) => {
            let v = int.base10_parse::<i64>()? as f64;
            return Ok(quote! { ::rusty::shared::Size::Px(#v) });
        }
        Lit::Float(float) => {
            let v = float.base10_parse::<f64>()?;
            return Ok(quote! { ::rusty::shared::Size::Px(#v) });
        }
        other => {
            return Err(syn::Error::new(
                other.span(),
                "expected a size such as `\"200px\"`, `\"50%\"` or `\"auto\"`",
            ))
        }
    };

    let span = lit.span();
    if text == "auto" {
        return Ok(quote! { ::rusty::shared::Size::Auto });
    }
    if let Some(px) = text.strip_suffix("px") {
        if let Ok(v) = px.trim().parse::<f64>() {
            return Ok(quote! { ::rusty::shared::Size::Px(#v) });
        }
    }
    if let Some(pct) = text.strip_suffix('%') {
        if let Ok(v) = pct.trim().parse::<f64>() {
            return Ok(quote! { ::rusty::shared::Size::Percent(#v) });
        }
    }

    let msg = format!("`{}` is not a size; use `200px`, `50%` or `auto`", text);
    Err(syn::Error::new(span, msg))
}

const ALIGN_VARIANTS: [&str; 4] = ["start", "center", "end", "stretch"];

const JUSTIFY_VARIANTS: [&str; 6] = [
    "start",
    "center",
    "end",
    "space-between",
    "space-around",
    "space-evenly",
];

const BUTTON_VARIANTS: [&str; 5] = ["primary", "secondary", "outline", "ghost", "danger"];

const TEXT_VARIANTS: [&str; 10] = [
    "block",
    "heading1",
    "heading2",
    "heading3",
    "heading4",
    "paragraph",
    "code",
    "markdown",
    "label",
    "caption",
];

const BADGE_VARIANTS: [&str; 3] = ["default", "outline", "dot"];

/// `variant` names a different enum per widget, so the element decides.
fn variant_arg(element: &Ident, value: &AttrValue) -> syn::Result<TokenStream> {
    match element.to_string().as_str() {
        "Button" => enum_arg(value, "ButtonVariant", &BUTTON_VARIANTS),
        "TextBlock" => enum_arg(value, "TextVariant", &TEXT_VARIANTS),
        "Badge" => enum_arg(value, "BadgeVariant", &BADGE_VARIANTS),
        _ => Ok(str_arg(value)),
    }
}

/// The module a variant enum lives in.
///
/// `widgets/mod.rs` re-exports the widget structs but not every variant enum
/// (`TextVariant`, `ButtonVariant` and `BadgeVariant` are all reachable only
/// through their defining module), so lowering emits the full path. The crate's
/// contract is to call existing code, not to add re-exports to `rusty::widgets`.
fn variant_module(enum_name: &str) -> Option<&'static str> {
    match enum_name {
        "ButtonVariant" => Some("button"),
        "TextVariant" => Some("text"),
        "BadgeVariant" => Some("badge"),
        _ => None,
    }
}

/// Map a kebab-or-snake spelling onto an enum variant path.
///
/// `justify="space-between"` and `justify="space_between"` both reach
/// `Justify::SpaceBetween`, because the wire form is camelCase and neither
/// spelling is more obviously right than the other in markup.
fn enum_arg(value: &AttrValue, enum_name: &str, variants: &[&str]) -> syn::Result<TokenStream> {
    let lit = match value {
        AttrValue::Expr(expr) => return Ok(quote! { #expr }),
        AttrValue::Literal(lit) => lit,
    };

    let Lit::Str(s) = lit else {
        let msg = format!("expected a `{}` name as a string literal", enum_name);
        return Err(syn::Error::new(lit.span(), msg));
    };

    let text = s.value();
    let normalized = text.replace('_', "-").to_lowercase();

    let Some(matched) = variants.iter().find(|v| **v == normalized) else {
        let msg = format!(
            "unknown {} `{}`; expected one of {}",
            enum_name,
            text,
            variants.join(", ")
        );
        return Err(syn::Error::new(s.span(), msg));
    };

    let span = s.span();
    let enum_ident = Ident::new(enum_name, span);
    let variant = Ident::new(&to_pascal_case(matched), span);
    let path = enum_path(enum_name, span);
    Ok(quote! { #path::#enum_ident::#variant })
}

/// `Align`/`Justify` are re-exported from `shared`; the per-widget variant enums
/// are not re-exported from `widgets`, so they need their defining module.
fn enum_path(enum_name: &str, span: Span) -> TokenStream {
    match variant_module(enum_name) {
        Some(module) => {
            let module = Ident::new(module, span);
            quote! { ::rusty::widgets::#module }
        }
        None => quote! { ::rusty::shared },
    }
}

fn to_pascal_case(kebab: &str) -> String {
    kebab
        .split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// Read a required string-literal attribute, e.g. `direction`.
fn string_literal(attr: &Attribute, name: &str) -> syn::Result<String> {
    match &attr.value {
        AttrValue::Literal(Lit::Str(s)) => Ok(s.value()),
        other => {
            let msg = format!("`{}` must be a string literal", name);
            Err(syn::Error::new(other.span(), msg))
        }
    }
}

/// Wrap a lowered root so the macro is usable directly as a `build()` return value.
pub fn root_tokens(root: &ElementNode) -> syn::Result<TokenStream> {
    let inner = element_tokens(root)?;
    Ok(quote! { ::rusty::views::Element::from(#inner) })
}
