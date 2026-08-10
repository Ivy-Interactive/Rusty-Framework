//! Turning one attribute string into the argument a builder takes.
//!
//! Two steps, deliberately separate:
//!
//! 1. [`resolve`] strips XAML's markup extensions, so `{Binding Count}` becomes
//!    whatever the [`XamlContext`] holds under `Count`. The result is a
//!    `serde_json::Value`, which is also what a widget's props serialize to.
//! 2. The `as_*` coercions turn that value into an `f64`, a `Size`, an enum, and
//!    so on. Enums go through serde rather than hand-written match arms: the
//!    variant list lives on the enum in `rusty::shared`, and duplicating it here
//!    is how the two drift apart.

use serde::de::DeserializeOwned;
use serde_json::Value;

use rusty::shared::{Align, Color, Justify, Size};

use crate::context::XamlContext;
use crate::error::{Position, XamlError};

/// Where in the document a value came from, carried so a coercion failure can
/// name the element and attribute it belongs to without threading the whole
/// document into every helper.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Site<'a> {
    pub element: &'a str,
    pub attribute: &'a str,
    pub pos: Position,
}

impl Site<'_> {
    pub(crate) fn unsupported(
        &self,
        value: impl Into<String>,
        reason: impl Into<String>,
    ) -> XamlError {
        XamlError::UnsupportedValue {
            element: self.element.to_string(),
            attribute: self.attribute.to_string(),
            value: value.into(),
            reason: reason.into(),
            pos: self.pos,
        }
    }
}

/// Resolve an attribute string, expanding a `{Binding ..}` markup extension
/// against `ctx`.
///
/// Resolution happens once, here, at parse time — the returned tree is a
/// snapshot. See the crate docs for what that means for re-rendering.
pub(crate) fn resolve(raw: &str, ctx: &XamlContext, site: Site<'_>) -> Result<Value, XamlError> {
    // `{}` is XAML's escape for a literal opening brace, so `{}{Binding x}` is
    // the text `{Binding x}`. It has to be checked before the `{` branch.
    if let Some(literal) = raw.strip_prefix("{}") {
        return Ok(Value::String(literal.to_string()));
    }

    let Some(inner) = raw
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
    else {
        if raw.starts_with('{') {
            return Err(unsupported_extension(&site, raw));
        }
        return Ok(Value::String(raw.to_string()));
    };

    let inner = inner.trim();
    let Some(rest) = inner.strip_prefix("Binding") else {
        return Err(unsupported_extension(&site, raw));
    };
    // `{BindingSource ..}` is not `{Binding ..}` with a path of `Source ..`.
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return Err(unsupported_extension(&site, raw));
    }

    // `{Binding Count}` and `{Binding Path=Count}` are the same binding.
    let path = rest.trim();
    let path = path.strip_prefix("Path=").unwrap_or(path).trim();
    if path.is_empty() {
        return Err(site.unsupported(
            raw,
            "a binding needs a path, e.g. `{Binding Count}`; there is no data context to bind to",
        ));
    }

    ctx.value_of(path)
        .cloned()
        .ok_or_else(|| XamlError::UnresolvedBinding {
            element: site.element.to_string(),
            attribute: site.attribute.to_string(),
            path: path.to_string(),
            pos: site.pos,
        })
}

fn unsupported_extension(site: &Site<'_>, raw: &str) -> XamlError {
    XamlError::UnsupportedMarkupExtension {
        element: site.element.to_string(),
        attribute: site.attribute.to_string(),
        value: raw.to_string(),
        pos: site.pos,
    }
}

/// The text form of a resolved value.
///
/// A bound number reaches a `&str` slot as its digits rather than as JSON, so
/// `Text="{Binding Count}"` with `Count = 3` renders `3`, not `"3"`.
pub(crate) fn as_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

pub(crate) fn as_f64(value: &Value, site: Site<'_>) -> Result<f64, XamlError> {
    if let Some(number) = value.as_f64() {
        return Ok(number);
    }
    let text = as_text(value);
    text.trim()
        .parse::<f64>()
        .map_err(|_| site.unsupported(text, "expected a number"))
}

pub(crate) fn as_usize(value: &Value, site: Site<'_>) -> Result<usize, XamlError> {
    if let Some(number) = value.as_u64() {
        return Ok(number as usize);
    }
    let text = as_text(value);
    text.trim()
        .parse::<usize>()
        .map_err(|_| site.unsupported(text, "expected a non-negative whole number"))
}

/// XAML writes booleans `True` / `False`; the wire form is `true` / `false`.
pub(crate) fn as_bool(value: &Value, site: Site<'_>) -> Result<bool, XamlError> {
    if let Some(flag) = value.as_bool() {
        return Ok(flag);
    }
    let text = as_text(value);
    match text.trim() {
        t if t.eq_ignore_ascii_case("true") => Ok(true),
        t if t.eq_ignore_ascii_case("false") => Ok(false),
        _ => Err(site.unsupported(text, "expected `True` or `False`")),
    }
}

/// `Width="120"`, `Width="240px"`, `Width="50%"` and `Width="Auto"`.
///
/// `Size::parse_css` covers the last three. The bare number is XAML's own
/// spelling (device-independent pixels) and `parse_css` rejects it, deliberately,
/// because it also deserializes the wire format — so the fallback lives here.
/// Star sizing (`*`, `2*`) has no `Size` variant and is rejected rather than
/// silently rounded to something else.
pub(crate) fn as_size(value: &Value, site: Site<'_>) -> Result<Size, XamlError> {
    let text = as_text(value);
    if let Some(size) = Size::parse_css(&text) {
        return Ok(size);
    }
    if let Ok(px) = text.trim().parse::<f64>() {
        return Ok(Size::Px(px));
    }
    Err(site.unsupported(
        text,
        "expected a length such as `120`, `240px`, `50%` or `Auto`; \
         XAML star sizing is not supported",
    ))
}

pub(crate) fn as_color(value: &Value, site: Site<'_>) -> Result<Color, XamlError> {
    let raw = as_text(value);
    serde_json::from_value::<Color>(Value::String(camel_case(&raw)))
        .map_err(|err| site.unsupported(raw, err.to_string()))
}

/// `HorizontalAlignment` in XAML terms, `align` in Rusty terms.
///
/// `Left` / `Right` are XAML's words for the same thing `Align::Start` / `End`
/// mean, so they are translated before serde sees the value; every other word
/// goes straight to the enum, which stays the single source of truth for the
/// variant set.
pub(crate) fn as_align(value: &Value, site: Site<'_>) -> Result<Align, XamlError> {
    let raw = as_text(value);
    let mapped = match raw.as_str() {
        "Left" => "Start",
        "Right" => "End",
        other => other,
    };
    as_enum_from(mapped, &raw, site)
}

/// `VerticalAlignment` in XAML terms, `justify` in Rusty terms.
pub(crate) fn as_justify(value: &Value, site: Site<'_>) -> Result<Justify, XamlError> {
    let raw = as_text(value);
    let mapped = match raw.as_str() {
        "Top" => "Start",
        "Bottom" => "End",
        other => other,
    };
    as_enum_from(mapped, &raw, site)
}

/// A PascalCase XAML value onto a `#[serde(rename_all = "camelCase")]` variant.
pub(crate) fn as_enum<T: DeserializeOwned>(value: &Value, site: Site<'_>) -> Result<T, XamlError> {
    let raw = as_text(value);
    as_enum_from(&raw, &raw, site)
}

fn as_enum_from<T: DeserializeOwned>(
    text: &str,
    raw: &str,
    site: Site<'_>,
) -> Result<T, XamlError> {
    serde_json::from_value::<T>(Value::String(camel_case(text)))
        .map_err(|err| site.unsupported(raw, err.to_string()))
}

/// `SpaceBetween` -> `spaceBetween`, `Heading1` -> `heading1`.
///
/// Lowercasing the first character is the whole conversion: the wire names are
/// camelCase of the same words, so anything more (splitting on case boundaries,
/// say) would break `Heading1`.
fn camel_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty::widgets::text::TextVariant;

    fn site() -> Site<'static> {
        Site {
            element: "TextBlock",
            attribute: "Text",
            pos: Position::new(1, 2),
        }
    }

    fn resolved(raw: &str, ctx: &XamlContext) -> Result<Value, XamlError> {
        resolve(raw, ctx, site())
    }

    #[test]
    fn plain_text_passes_through() {
        let ctx = XamlContext::new();
        assert_eq!(
            resolved("Dashboard", &ctx).unwrap(),
            Value::from("Dashboard")
        );
        assert_eq!(resolved("", &ctx).unwrap(), Value::from(""));
    }

    #[test]
    fn binding_and_explicit_path_resolve_identically() {
        let ctx = XamlContext::new().value("Count", 3);

        let short = resolved("{Binding Count}", &ctx).unwrap();
        let explicit = resolved("{Binding Path=Count}", &ctx).unwrap();

        assert_eq!(short, Value::from(3));
        assert_eq!(short, explicit);
    }

    #[test]
    fn binding_tolerates_surrounding_whitespace() {
        let ctx = XamlContext::new().value("Title", "Dashboard");
        assert_eq!(
            resolved("{ Binding  Title }", &ctx).unwrap(),
            Value::from("Dashboard")
        );
    }

    #[test]
    fn brace_escape_yields_the_literal_braces() {
        let ctx = XamlContext::new();
        assert_eq!(
            resolved("{}{Binding literal}", &ctx).unwrap(),
            Value::from("{Binding literal}")
        );
    }

    #[test]
    fn other_markup_extensions_are_rejected() {
        let ctx = XamlContext::new();

        for raw in [
            "{StaticResource Brush}",
            "{TemplateBinding Width}",
            "{x:Static Colors.Red}",
            "{BindingSource Count}",
            "{Binding Count",
        ] {
            let err = resolved(raw, &ctx).unwrap_err();
            assert!(
                matches!(err, XamlError::UnsupportedMarkupExtension { .. }),
                "{raw} produced {err:?}"
            );
            assert!(err.to_string().contains(raw), "{}", err);
        }
    }

    #[test]
    fn an_empty_binding_path_is_rejected() {
        let ctx = XamlContext::new();
        let err = resolved("{Binding}", &ctx).unwrap_err();
        assert!(matches!(err, XamlError::UnsupportedValue { .. }), "{err:?}");
    }

    #[test]
    fn an_unresolved_binding_is_an_error_not_an_empty_string() {
        let ctx = XamlContext::new().value("Other", 1);
        let err = resolved("{Binding Count}", &ctx).unwrap_err();

        assert!(
            matches!(&err, XamlError::UnresolvedBinding { path, .. } if path == "Count"),
            "{err:?}"
        );
        assert!(err.to_string().contains("Count"), "{}", err);
    }

    #[test]
    fn text_of_a_bound_number_is_its_digits() {
        assert_eq!(as_text(&Value::from(3)), "3");
        assert_eq!(as_text(&Value::from(1.5)), "1.5");
        assert_eq!(as_text(&Value::from(true)), "true");
        assert_eq!(as_text(&Value::from("x")), "x");
        assert_eq!(as_text(&Value::Null), "");
    }

    #[test]
    fn numbers_come_from_json_or_from_text() {
        assert_eq!(as_f64(&Value::from(1.5), site()).unwrap(), 1.5);
        assert_eq!(as_f64(&Value::from("2.5"), site()).unwrap(), 2.5);
        assert_eq!(as_usize(&Value::from(3), site()).unwrap(), 3);
        assert_eq!(as_usize(&Value::from(" 4 "), site()).unwrap(), 4);

        assert!(as_f64(&Value::from("wide"), site()).is_err());
        assert!(as_usize(&Value::from("-1"), site()).is_err());
    }

    #[test]
    fn booleans_accept_xaml_casing() {
        assert!(as_bool(&Value::from("True"), site()).unwrap());
        assert!(!as_bool(&Value::from("False"), site()).unwrap());
        assert!(as_bool(&Value::from(true), site()).unwrap());
        assert!(as_bool(&Value::from("yes"), site()).is_err());
    }

    #[test]
    fn sizes_accept_bare_pixels_percent_and_auto() {
        assert_eq!(
            as_size(&Value::from("120"), site()).unwrap(),
            Size::Px(120.0)
        );
        assert_eq!(
            as_size(&Value::from("240px"), site()).unwrap(),
            Size::Px(240.0)
        );
        assert_eq!(
            as_size(&Value::from("50%"), site()).unwrap(),
            Size::Percent(50.0)
        );
        assert_eq!(as_size(&Value::from("Auto"), site()).unwrap(), Size::Auto);
        assert_eq!(as_size(&Value::from(120), site()).unwrap(), Size::Px(120.0));
    }

    #[test]
    fn star_sizing_is_rejected() {
        for raw in ["*", "2*", "1.5*"] {
            let err = as_size(&Value::from(raw), site()).unwrap_err();
            assert!(matches!(err, XamlError::UnsupportedValue { .. }), "{err:?}");
            assert!(err.to_string().contains("star sizing"), "{}", err);
        }
    }

    #[test]
    fn enums_resolve_through_serde() {
        assert_eq!(
            as_enum::<TextVariant>(&Value::from("Heading1"), site()).unwrap(),
            TextVariant::Heading1
        );
        assert_eq!(
            as_align(&Value::from("Left"), site()).unwrap(),
            Align::Start
        );
        assert_eq!(as_align(&Value::from("Right"), site()).unwrap(), Align::End);
        assert_eq!(
            as_align(&Value::from("Stretch"), site()).unwrap(),
            Align::Stretch
        );
        assert_eq!(
            as_justify(&Value::from("Top"), site()).unwrap(),
            Justify::Start
        );
        assert_eq!(
            as_justify(&Value::from("SpaceBetween"), site()).unwrap(),
            Justify::SpaceBetween
        );
    }

    #[test]
    fn an_unknown_enum_value_reports_the_accepted_variants() {
        let err = as_align(&Value::from("Middle"), site()).unwrap_err();
        let message = err.to_string();

        assert!(message.contains("Middle"), "{}", message);
        assert!(message.contains("stretch"), "{}", message);
    }

    #[test]
    fn colors_accept_named_hex_and_rgba() {
        assert_eq!(
            as_color(&Value::from("Primary"), site()).unwrap(),
            Color::Named(rusty::shared::NamedColor::Primary)
        );
        assert_eq!(
            as_color(&Value::from("#ff0000"), site()).unwrap(),
            Color::Hex("#ff0000".to_string())
        );
        assert!(as_color(&Value::from("Chartreuse"), site()).is_err());
    }
}
