use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro for the Widget trait.
///
/// Automatically implements `WidgetData` for a struct, generating
/// `widget_type()`, `to_json()`, and `clone_box()` methods.
///
/// Fields marked with `#[prop]` are included in serialization.
/// Fields marked with `#[event]` are skipped in serialization but
/// registered as event handlers.
///
/// # Example
///
/// ```ignore
/// use rusty_macros::Widget;
///
/// #[derive(Widget, Clone, Debug)]
/// struct MyWidget {
///     #[prop]
///     title: String,
///     #[prop]
///     disabled: bool,
///     #[event]
///     on_click: Option<Arc<dyn Fn() + Send + Sync>>,
/// }
/// ```
#[proc_macro_derive(Widget, attributes(prop, event))]
pub fn derive_widget(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let widget_type = to_snake_case(&name.to_string());

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

    let prop_fields: Vec<_> = fields
        .iter()
        .filter(|f| f.attrs.iter().any(|a| a.path().is_ident("prop")))
        .collect();

    let json_fields: Vec<_> = prop_fields
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let json_key = to_camel_case(&field_name.to_string());
            quote! {
                map.insert(#json_key.to_string(), serde_json::to_value(&self.#field_name).unwrap_or_default());
            }
        })
        .collect();

    let expanded = quote! {
        impl crate::views::view::WidgetData for #name {
            fn widget_type(&self) -> &str {
                #widget_type
            }

            fn to_json(&self) -> serde_json::Value {
                let mut map = serde_json::Map::new();
                map.insert("type".to_string(), serde_json::Value::String(#widget_type.to_string()));
                #(#json_fields)*
                serde_json::Value::Object(map)
            }

            fn clone_box(&self) -> Box<dyn crate::views::view::WidgetData> {
                Box::new(self.clone())
            }
        }
    };

    TokenStream::from(expanded)
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
