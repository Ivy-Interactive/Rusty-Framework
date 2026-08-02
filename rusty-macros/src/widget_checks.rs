//! Shape diagnostics for `#[derive(Widget)]`.
//!
//! The derive already fails on two shapes ("only supports named fields", "only
//! supports structs"). Everything else it gets wrong either surfaces as a type
//! error deep inside generated code — `E0599: no method named is_some found for
//! Arc<dyn Fn()>` tells an author nothing about the `#[event]` attribute they
//! put on a non-`Option` field — or, in two cases, compiles clean and misbehaves
//! at runtime:
//!
//! * a `Vec<Element>` field not named `children` gets no `children_mut`, so
//!   `Element::assign_ids` never descends into it and its whole subtree loses
//!   IDs and event registrations;
//! * a struct with `#[event]` fields but no `#[prop]` fields and no `id`
//!   serializes to just `{"type": ".."}`.
//!
//! Every check is syntactic. A proc macro cannot resolve a type alias, so the
//! checks flag only unambiguous shapes and leave the existing type errors to
//! cover the rest.

use syn::spanned::Spanned;
use syn::{Field, Fields, Ident};

/// Attribute names that mark a field as a container the derive walks into.
///
/// `children` / `child` / `footer` do not exist on the derive yet — plan 00093
/// adds them. Listing them now makes the container check correct both before and
/// after that lands: a field carrying one of them is never flagged.
const CONTAINER_ATTRS: &[&str] = &["children", "child", "footer"];

/// Collect every diagnostic for a named-field struct, folded into one
/// `syn::Error` so all problems report at once.
pub(crate) fn check_struct(name: &Ident, fields: &Fields) -> syn::Result<()> {
    let named = match fields {
        Fields::Named(named) => &named.named,
        _ => return Ok(()),
    };

    let mut errors: Vec<syn::Error> = Vec::new();

    let mut prop_count = 0usize;
    let mut event_count = 0usize;
    let mut has_id = false;

    for field in named {
        let is_prop = has_attr(field, "prop");
        let is_event = has_attr(field, "event");
        let field_name = field.ident.as_ref().expect("named fields have idents");

        if is_prop {
            prop_count += 1;
        }
        if is_event {
            event_count += 1;
        }
        if field_name == "id" {
            has_id = true;
        }

        // `#[prop]` and `#[event]` on one field: the prop arm serializes the
        // field, and `Arc<dyn Fn(..)>` is not `Serialize` (today's E0277).
        if is_prop && is_event {
            errors.push(syn::Error::new(
                field.span(),
                format!(
                    "`{field_name}` carries both `#[prop]` and `#[event]`. `#[prop]` serializes \
                     the field and an event callback is not serializable — pick one."
                ),
            ));
        }

        // `#[event]` on a non-`Option` field: the derive emits
        // `self.<field>.is_some()` (today's E0599).
        if is_event && !is_prop && !is_option(&field.ty) {
            errors.push(syn::Error::new(
                field.ty.span(),
                format!(
                    "`#[event]` field `{field_name}` must be an `Option<..>`; the derive emits \
                     `self.{field_name}.is_some()` to report the handler's presence. Wrap the \
                     type: `Option<{}>`.",
                    type_to_string(&field.ty)
                ),
            ));
        }

        // `id` drives `assign_id` / `get_id`, which need `Option<String>`
        // (today's E0308 plus E0599 on `as_deref`).
        if field_name == "id" && !is_option_of(&field.ty, "String") {
            errors.push(syn::Error::new(
                field.ty.span(),
                format!(
                    "the `id` field must be `Option<String>`; the derive generates \
                     `self.id = Some(id)` and `self.id.as_deref()`, which `{}` does not support.",
                    type_to_string(&field.ty)
                ),
            ));
        }

        // A container the derive cannot see. `children_mut` is gated on a field
        // literally named `children`, so any other name silently drops the
        // subtree.
        if is_element_container(&field.ty)
            && field_name != "children"
            && !CONTAINER_ATTRS.iter().any(|a| has_attr(field, a))
        {
            errors.push(syn::Error::new(
                field.span(),
                format!(
                    "`{field_name}: {}` is a container field the derive cannot see, so its \
                     descendants never get IDs or event handlers. Rename it to `children`, or \
                     hand-write `children_mut` instead of deriving it.",
                    type_to_string(&field.ty)
                ),
            ));
        }
    }

    // A struct that serializes to just `{"type": ".."}` plus `has<Event>` flags
    // carries no data the client can render.
    if event_count > 0 && prop_count == 0 && !has_id {
        errors.push(syn::Error::new(
            name.span(),
            format!(
                "`{name}` has {event_count} `#[event]` field(s) but no `#[prop]` fields and no \
                 `id`, so `to_json` emits only its type and `has<Event>` flags — the client \
                 receives no data and the handlers can never be addressed. Add `#[prop]` to the \
                 fields the client needs, or an `id: Option<String>` field."
            ),
        ));
    }

    let mut errors = errors.into_iter();
    match errors.next() {
        None => Ok(()),
        Some(mut combined) => {
            for error in errors {
                combined.combine(error);
            }
            Err(combined)
        }
    }
}

fn has_attr(field: &Field, name: &str) -> bool {
    field.attrs.iter().any(|a| a.path().is_ident(name))
}

/// The last path segment of a type, when the type is a plain path.
fn last_segment(ty: &syn::Type) -> Option<&syn::PathSegment> {
    match ty {
        syn::Type::Path(path) if path.qself.is_none() => path.path.segments.last(),
        _ => None,
    }
}

fn is_option(ty: &syn::Type) -> bool {
    last_segment(ty).is_some_and(|s| s.ident == "Option")
}

/// The single generic argument of a `Name<Arg>` type.
fn sole_generic_arg(segment: &syn::PathSegment) -> Option<&syn::Type> {
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    if args.args.len() != 1 {
        return None;
    }
    match args.args.first() {
        Some(syn::GenericArgument::Type(ty)) => Some(ty),
        _ => None,
    }
}

/// `Option<Inner>` where `Inner`'s last segment is `inner`.
fn is_option_of(ty: &syn::Type, inner: &str) -> bool {
    last_segment(ty)
        .filter(|s| s.ident == "Option")
        .and_then(sole_generic_arg)
        .and_then(last_segment)
        .is_some_and(|s| s.ident == inner)
}

/// `Vec<Element>` or `Option<Vec<Element>>`, matching `Element` on its last path
/// segment so `crate::views::view::Element` counts too.
fn is_element_container(ty: &syn::Type) -> bool {
    let Some(segment) = last_segment(ty) else {
        return false;
    };
    match segment.ident.to_string().as_str() {
        "Vec" => sole_generic_arg(segment)
            .and_then(last_segment)
            .is_some_and(|s| s.ident == "Element"),
        "Option" => sole_generic_arg(segment).is_some_and(is_element_container),
        _ => false,
    }
}

/// Render a type for a diagnostic, with the token spacing `quote` produces
/// collapsed so `Vec < Element >` reads as `Vec<Element>` and `Arc < dyn Fn () >`
/// reads as `Arc<dyn Fn()>`.
fn type_to_string(ty: &syn::Type) -> String {
    let rendered = quote::quote!(#ty).to_string();
    let mut out = String::with_capacity(rendered.len());
    let mut chars = rendered.chars().peekable();
    while let Some(ch) = chars.next() {
        // Drop a space that hugs punctuation. The two sides differ: `Fn ()`
        // closes up to `Fn()`, but `Fn() + Send` keeps the space after `)`.
        if ch == ' ' {
            let drop_before = chars.peek().is_some_and(|c| tight_before(*c));
            let drop_after = out.chars().last().is_some_and(tight_after);
            if drop_before || drop_after {
                continue;
            }
        }
        out.push(ch);
    }
    out
}

/// Punctuation that takes no space *before* it: `Vec <` , `Fn (` , `A ,`.
fn tight_before(ch: char) -> bool {
    matches!(ch, '<' | '>' | '(' | ')' | '[' | ']' | ',' | ':')
}

/// Punctuation that takes no space *after* it. Deliberately excludes the closing
/// brackets and the comma, which do take one (`Fn() + Send`, `A, B`).
fn tight_after(ch: char) -> bool {
    matches!(ch, '<' | '(' | '[' | ':')
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{Data, DeriveInput};

    /// Run the checks over a struct written as a string, returning each
    /// diagnostic's message.
    fn check(source: &str) -> Vec<String> {
        let input: DeriveInput = syn::parse_str(source).expect("fixture must parse");
        let Data::Struct(data) = &input.data else {
            panic!("fixture must be a struct");
        };
        match check_struct(&input.ident, &data.fields) {
            Ok(()) => Vec::new(),
            Err(error) => error.into_iter().map(|e| e.to_string()).collect(),
        }
    }

    #[test]
    fn a_well_formed_widget_is_clean() {
        assert!(check(
            "struct W {
                 id: Option<String>,
                 #[prop] title: String,
                 #[event] on_click: Option<Arc<dyn Fn() + Send + Sync>>,
                 children: Vec<Element>,
             }"
        )
        .is_empty());
    }

    #[test]
    fn misnamed_container_is_flagged() {
        let found = check("struct W { #[prop] n: u8, items: Vec<Element> }");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("`items: Vec<Element>` is a container field"),
            "{found:?}"
        );
    }

    #[test]
    fn misnamed_optional_container_is_flagged() {
        let found = check("struct W { #[prop] n: u8, extras: Option<Vec<Element>> }");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("`extras: Option<Vec<Element>>`"),
            "{found:?}"
        );
    }

    #[test]
    fn qualified_element_path_is_still_a_container() {
        let found = check("struct W { #[prop] n: u8, items: Vec<crate::views::view::Element> }");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("container field"), "{found:?}");
    }

    #[test]
    fn container_named_children_is_clean() {
        assert!(check("struct W { #[prop] n: u8, children: Vec<Element> }").is_empty());
    }

    #[test]
    fn container_with_plan_00093_attributes_is_clean() {
        // These attributes do not exist on the derive yet; the check must not
        // fight them once plan 00093 adds them.
        for attr in CONTAINER_ATTRS {
            let found = check(&format!(
                "struct W {{ #[prop] n: u8, #[{attr}] items: Vec<Element> }}"
            ));
            assert!(
                found.is_empty(),
                "#[{attr}] should suppress the container check: {found:?}"
            );
        }
    }

    #[test]
    fn vec_of_non_element_is_not_a_container() {
        assert!(check("struct W { #[prop] tags: Vec<String> }").is_empty());
    }

    #[test]
    fn event_only_struct_is_flagged() {
        let found = check("struct W { #[event] on_click: Option<Arc<dyn Fn()>> }");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("no `#[prop]` fields and no `id`"),
            "{found:?}"
        );
    }

    #[test]
    fn event_struct_with_id_is_clean() {
        assert!(
            check("struct W { id: Option<String>, #[event] on_click: Option<Arc<dyn Fn()>> }")
                .is_empty()
        );
    }

    #[test]
    fn struct_with_no_events_and_no_props_is_clean() {
        // A marker widget serializing to just its type is legitimate.
        assert!(check("struct W { internal: u8 }").is_empty());
    }

    #[test]
    fn prop_and_event_on_one_field_is_flagged() {
        let found =
            check("struct W { #[prop] #[event] on_click: Option<Arc<dyn Fn() + Send + Sync>> }");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("both `#[prop]` and `#[event]`"),
            "{found:?}"
        );
    }

    #[test]
    fn non_option_event_field_is_flagged() {
        let found = check("struct W { #[prop] n: u8, #[event] on_click: Arc<dyn Fn()> }");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("must be an `Option<..>`"), "{found:?}");
        assert!(found[0].contains("Option<Arc<dyn Fn()>>"), "{found:?}");
    }

    #[test]
    fn rendered_types_keep_word_spacing_and_drop_bracket_spacing() {
        // The suggested `Option<..>` wrapper is read by a human, so the
        // rendering has to survive `quote`'s uniform token spacing.
        let found =
            check("struct W { #[prop] n: u8, #[event] on_click: Arc<dyn Fn(u8) + Send + Sync> }");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("Option<Arc<dyn Fn(u8) + Send + Sync>>"),
            "{found:?}"
        );
    }

    #[test]
    fn bad_id_type_is_flagged() {
        let found = check("struct W { id: String, #[prop] n: u8 }");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("must be `Option<String>`"), "{found:?}");
    }

    #[test]
    fn option_id_of_wrong_inner_type_is_flagged() {
        let found = check("struct W { id: Option<u32>, #[prop] n: u8 }");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("must be `Option<String>`"), "{found:?}");
    }

    #[test]
    fn multiple_problems_report_together() {
        let found = check(
            "struct W {
                 id: String,
                 #[prop] #[event] on_click: Option<Arc<dyn Fn()>>,
                 items: Vec<Element>,
             }",
        );
        assert_eq!(found.len(), 3, "{found:?}");
    }

    #[test]
    fn tuple_struct_is_not_checked_here() {
        // `Fields::Unnamed` is rejected by the derive itself with "only
        // supports named fields"; these checks stay out of the way.
        assert!(check("struct W(u8);").is_empty());
    }
}
