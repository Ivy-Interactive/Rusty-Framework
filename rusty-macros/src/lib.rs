use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, DeriveInput, Field, Ident};

/// Derive macro for the `WidgetData` trait.
///
/// Generates `widget_type()`, `to_json()`, `clone_box()`, `assign_id()`,
/// `get_id()`, and — when the struct declares the relevant fields —
/// `register_events()`, `children_mut()`, `single_child_mut()` and
/// `footer_mut()`.
///
/// The `id` field (if present as `Option<String>`) is automatically included in
/// the JSON output and drives `assign_id`/`get_id`.
///
/// # Attributes
///
/// | Attribute | On | Effect |
/// | --- | --- | --- |
/// | `#[widget(type = "icon")]` | struct | Overrides the derived `widget_type` string. |
/// | `#[prop]` | field | Serialized into `to_json` under its camelCase key. |
/// | `#[prop(with = "path::to::fn")]` | field | Serializes `path(&self.field)` instead of the field. |
/// | `#[event]` | field | Emits `has<Field>` plus a no-payload `register_events` arm. |
/// | `#[event(arg = "value")]` | field | The handler receives `args["value"]`, deserialized. |
/// | `#[event(payload)]` | field | The handler receives the whole `args` object, deserialized. |
/// | `#[children]` | field | `children_mut` returns `Some(&mut self.field)`. |
/// | `#[child]` | field | Generates `single_child_mut`. |
/// | `#[footer]` | field | Generates `footer_mut` (works on `Vec` or `Option<Vec>`). |
///
/// Without `#[children]`, a field literally named `children` is still picked up.
///
/// The event name is derived from the field name by stripping a leading `on_`
/// and removing the remaining underscores (`on_cell_click` -> `cellclick`),
/// matching `EventName::as_str()`. `#[event(name = "...")]` overrides it.
///
/// Malformed payloads are dropped: an `arg`/`payload` arm that fails to
/// deserialize does not invoke the handler and does not panic.
///
/// # Example
///
/// ```ignore
/// use rusty_macros::Widget;
///
/// #[derive(Widget, Clone, Debug)]
/// #[widget(type = "icon")]
/// struct MyWidget {
///     id: Option<String>,
///     #[prop]
///     title: String,
///     #[prop(with = "crate::shared::size_css")]
///     width: Option<Size>,
///     #[children]
///     items: Vec<Element>,
///     #[event]
///     on_click: Option<Arc<dyn Fn() + Send + Sync>>,
///     #[event(arg = "value")]
///     on_change: Option<Arc<dyn Fn(String) + Send + Sync>>,
/// }
/// ```
#[proc_macro_derive(Widget, attributes(widget, prop, event, children, child, footer))]
pub fn derive_widget(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let widget_type = match widget_type_override(&input.attrs) {
        Ok(Some(explicit)) => explicit,
        Ok(None) => to_snake_case(&name.to_string()),
        Err(err) => return err.to_compile_error().into(),
    };

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(name, "Widget derive only supports named fields")
                    .to_compile_error()
                    .into()
            }
        },
        _ => {
            return syn::Error::new_spanned(name, "Widget derive only supports structs")
                .to_compile_error()
                .into()
        }
    };

    let prop_fields: Vec<_> = fields.iter().filter(|f| has_attr(f, "prop")).collect();

    let event_specs: Vec<EventSpec> = match fields
        .iter()
        .filter(|f| has_attr(f, "event"))
        .map(EventSpec::parse)
        .collect()
    {
        Ok(specs) => specs,
        Err(err) => return err.to_compile_error().into(),
    };

    let has_id_field = fields
        .iter()
        .any(|f| f.ident.as_ref().is_some_and(|i| i == "id"));

    let json_fields: Vec<_> = match prop_fields
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let json_key = to_camel_case(&field_name.to_string());
            let value = match prop_with(f)? {
                Some(path) => quote! { #path(&self.#field_name) },
                None => quote! { &self.#field_name },
            };
            Ok(quote! {
                map.insert(#json_key.to_string(), serde_json::to_value(#value).unwrap_or_default());
            })
        })
        .collect::<syn::Result<Vec<_>>>()
    {
        Ok(fields) => fields,
        Err(err) => return err.to_compile_error().into(),
    };

    // Generate "has<EventName>" boolean entries for event fields
    let event_has_fields: Vec<_> = event_specs
        .iter()
        .map(|spec| {
            let field_name = &spec.field;
            let has_key = format!("has{}", to_pascal_case(&field_name.to_string()));
            quote! {
                map.insert(#has_key.to_string(), serde_json::Value::Bool(self.#field_name.is_some()));
            }
        })
        .collect();

    let id_field = if has_id_field {
        quote! {
            map.insert("id".to_string(), serde_json::to_value(&self.id).unwrap_or_default());
        }
    } else {
        quote! {}
    };

    let assign_id_impl = if has_id_field {
        quote! {
            fn assign_id(&mut self, id: String) {
                self.id = Some(id);
            }

            fn get_id(&self) -> Option<&str> {
                self.id.as_deref()
            }
        }
    } else {
        quote! {
            fn assign_id(&mut self, _id: String) {}
            fn get_id(&self) -> Option<&str> { None }
        }
    };

    // `#[event]` fields wire their stored closure into the registry. Omitted
    // entirely when there are none, so the trait's no-op default keeps applying.
    let register_events_impl = if event_specs.is_empty() {
        quote! {}
    } else {
        let arms = event_specs.iter().map(EventSpec::register_arm);
        quote! {
            fn register_events(
                &self,
                widget_id: &str,
                registry: &mut crate::core::event_registry::EventRegistry,
            ) {
                #(#arms)*
            }
        }
    };

    // The container field defaults to one literally named `children`, so the
    // pre-attribute code path stays source-compatible.
    let children_field = fields
        .iter()
        .find(|f| has_attr(f, "children"))
        .or_else(|| {
            fields
                .iter()
                .find(|f| f.ident.as_ref().is_some_and(|i| i == "children"))
        })
        .and_then(|f| f.ident.clone());

    let children_mut_impl = match children_field {
        Some(field) => quote! {
            fn children_mut(&mut self) -> Option<&mut Vec<crate::views::view::Element>> {
                Some(&mut self.#field)
            }
        },
        None => quote! {},
    };

    let single_child_mut_impl = match fields.iter().find(|f| has_attr(f, "child")) {
        Some(f) => {
            let field = f.ident.as_ref().unwrap();
            quote! {
                fn single_child_mut(&mut self) -> Option<&mut crate::views::view::Element> {
                    Some(&mut self.#field)
                }
            }
        }
        None => quote! {},
    };

    // `footer` is `Option<Vec<Element>>` on Card and a bare `Vec<Element>`
    // elsewhere, so accept either shape.
    let footer_mut_impl = match fields.iter().find(|f| has_attr(f, "footer")) {
        Some(f) => {
            let field = f.ident.as_ref().unwrap();
            let body = if is_option(&f.ty) {
                quote! { self.#field.as_mut() }
            } else {
                quote! { Some(&mut self.#field) }
            };
            quote! {
                fn footer_mut(&mut self) -> Option<&mut Vec<crate::views::view::Element>> {
                    #body
                }
            }
        }
        None => quote! {},
    };

    let expanded = quote! {
        impl crate::views::view::WidgetData for #name {
            fn widget_type(&self) -> &str {
                #widget_type
            }

            fn to_json(&self) -> serde_json::Value {
                let mut map = serde_json::Map::new();
                map.insert("type".to_string(), serde_json::Value::String(#widget_type.to_string()));
                #id_field
                #(#json_fields)*
                #(#event_has_fields)*
                serde_json::Value::Object(map)
            }

            fn clone_box(&self) -> Box<dyn crate::views::view::WidgetData> {
                Box::new(self.clone())
            }

            #assign_id_impl
            #register_events_impl
            #children_mut_impl
            #single_child_mut_impl
            #footer_mut_impl
        }
    };

    TokenStream::from(expanded)
}

/// How an `#[event]` field's payload reaches its handler.
enum EventArg {
    /// `Fn()` — no payload.
    None,
    /// `Fn(T)` — deserialize `args[key]` into `T`.
    Key(String),
    /// `Fn(T)` — deserialize the whole `args` object into `T`.
    Payload,
}

/// One `#[event]` field: which field holds the handler, the wire event name,
/// and how the JSON payload is delivered.
struct EventSpec {
    field: Ident,
    name: String,
    arg: EventArg,
}

impl EventSpec {
    fn parse(field: &Field) -> syn::Result<Self> {
        let ident = field.ident.clone().unwrap();
        let mut name = default_event_name(&ident.to_string());
        let mut arg = EventArg::None;

        for attr in field.attrs.iter().filter(|a| a.path().is_ident("event")) {
            // A bare `#[event]` has no parenthesized body to parse.
            if matches!(attr.meta, syn::Meta::Path(_)) {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("arg") {
                    arg = EventArg::Key(meta.value()?.parse::<syn::LitStr>()?.value());
                    Ok(())
                } else if meta.path.is_ident("payload") {
                    arg = EventArg::Payload;
                    Ok(())
                } else if meta.path.is_ident("name") {
                    name = meta.value()?.parse::<syn::LitStr>()?.value();
                    Ok(())
                } else {
                    Err(meta.error("expected `arg = \"...\"`, `payload`, or `name = \"...\"`"))
                }
            })?;
        }

        Ok(EventSpec {
            field: ident,
            name,
            arg,
        })
    }

    /// The `register_events` body for this field: clone the `Arc` out of the
    /// `Option` and hand the registry a closure that decodes the payload.
    fn register_arm(&self) -> proc_macro2::TokenStream {
        let field = &self.field;
        let event_name = &self.name;
        let closure = match &self.arg {
            EventArg::None => quote! {
                move |_args: serde_json::Value| handler()
            },
            EventArg::Key(key) => quote! {
                move |args: serde_json::Value| {
                    let raw = args.get(#key).cloned().unwrap_or(serde_json::Value::Null);
                    if let Ok(value) = serde_json::from_value(raw) {
                        handler(value);
                    }
                }
            },
            EventArg::Payload => quote! {
                move |args: serde_json::Value| {
                    if let Ok(value) = serde_json::from_value(args) {
                        handler(value);
                    }
                }
            },
        };

        quote! {
            if let Some(handler) = &self.#field {
                let handler = handler.clone();
                registry.register(widget_id, #event_name, std::sync::Arc::new(#closure));
            }
        }
    }
}

fn has_attr(field: &Field, name: &str) -> bool {
    field.attrs.iter().any(|a| a.path().is_ident(name))
}

/// `on_cell_click` -> `cellclick`, matching `EventName::as_str()`.
fn default_event_name(field: &str) -> String {
    field.strip_prefix("on_").unwrap_or(field).replace('_', "")
}

/// The `#[prop(with = "path")]` serialization hook, if any.
fn prop_with(field: &Field) -> syn::Result<Option<syn::Path>> {
    let mut with = None;
    for attr in field.attrs.iter().filter(|a| a.path().is_ident("prop")) {
        if matches!(attr.meta, syn::Meta::Path(_)) {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("with") {
                let lit: syn::LitStr = meta.value()?.parse()?;
                with = Some(lit.parse()?);
                Ok(())
            } else {
                Err(meta.error("expected `with = \"path::to::fn\"`"))
            }
        })?;
    }
    Ok(with)
}

/// The `#[widget(type = "...")]` wire-name override, if any.
fn widget_type_override(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    let mut explicit = None;
    for attr in attrs.iter().filter(|a| a.path().is_ident("widget")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("type") {
                explicit = Some(meta.value()?.parse::<syn::LitStr>()?.value());
                Ok(())
            } else {
                Err(meta.error("expected `type = \"...\"`"))
            }
        })?;
    }
    Ok(explicit)
}

/// Whether a type is spelled `Option<..>`, to tell an `Option<Vec<Element>>`
/// footer from a bare `Vec<Element>` one.
fn is_option(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "Option"),
        _ => false,
    }
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

/// Convert snake_case to PascalCase (e.g., on_click -> OnClick)
fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;
    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}
