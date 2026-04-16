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
/// generate `has_<event_name>` boolean fields in the JSON output
/// and can be registered via the `build()` method.
///
/// The `id` field (if present as `Option<String>`) is automatically
/// included in the JSON output.
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

    let event_fields: Vec<_> = fields
        .iter()
        .filter(|f| f.attrs.iter().any(|a| a.path().is_ident("event")))
        .collect();

    let has_id_field = fields
        .iter()
        .any(|f| f.ident.as_ref().is_some_and(|i| i == "id"));

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

    // Generate "has<EventName>" boolean entries for event fields
    let event_has_fields: Vec<_> = event_fields
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let field_str = field_name.to_string();
            // Convert on_click -> hasOnClick, on_change -> hasOnChange
            let has_key = format!("has{}", to_pascal_case(&field_str));
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

    // Detect children field (Vec<Element>) for container widgets
    let has_children_field = fields
        .iter()
        .any(|f| f.ident.as_ref().is_some_and(|i| i == "children"));

    let children_mut_impl = if has_children_field {
        quote! {
            fn children_mut(&mut self) -> Option<&mut Vec<crate::views::view::Element>> {
                Some(&mut self.children)
            }
        }
    } else {
        quote! {}
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
            #children_mut_impl
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
