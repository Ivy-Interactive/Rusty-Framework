use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemImpl};

mod hook_rules;
mod widget_checks;

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
    match expand_widget(&input) {
        Ok(expanded) => expanded.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// The derive's body, with every diagnostic routed through `syn::Result`.
///
/// Shape errors and the [`widget_checks`] diagnostics are collected rather than
/// returned one at a time, so an author with three problems sees three messages
/// instead of fixing them one compile at a time.
fn expand_widget(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let widget_type = to_snake_case(&name.to_string());

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "Widget derive only supports named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "Widget derive only supports structs",
            ))
        }
    };

    // Bail before generating anything: the generated code is what turns these
    // shapes into the type errors these diagnostics replace.
    if let syn::Data::Struct(data) = &input.data {
        widget_checks::check_struct(name, &data.fields)?;
    }

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

    Ok(quote! {
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
    })
}

/// Check an `impl View for X` block against the framework's hook invariants.
///
/// Hook state lives in slots keyed by *call index*
/// (`BuildContext::next_hook_index`), the same ordering rule as React and Ivy.
/// Two ways to break it are invisible to the compiler, and this attribute makes
/// both a compile error:
///
/// * **`conditional_hooks`** — a hook called inside an `if`, a `match` arm, a
///   loop, a closure or an `async` block shifts every later hook's slot, so
///   `get_or_init_state` starts returning another hook's value.
/// * **`set_during_build`** — `State::set` / `State::update` called
///   synchronously in `build` requests a rebuild of the view that is currently
///   building: an unconditional rebuild loop. `use_ref` returns the same
///   `State<T>` with rebuilds disabled, so the rule tracks which hook each
///   binding came from instead of trusting the method name.
///
/// Both rules are syntactic, so both can be switched off for one impl block:
///
/// ```ignore
/// #[rusty::view(allow(conditional_hooks))]
/// impl View for MyApp {
///     fn build(&self, ctx: &mut BuildContext) -> Element { /* ... */ }
/// }
/// ```
///
/// The attribute never changes the code it is applied to — it re-emits the impl
/// block verbatim and appends any diagnostics, so a violation does not also
/// produce "the trait `View` is not implemented" noise.
#[proc_macro_attribute]
pub fn view(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);

    let config = match hook_rules::parse_rule_config(attr.into()) {
        Ok(config) => config,
        // An unparseable attribute argument still emits the impl block, for the
        // same reason a rule violation does.
        Err(error) => {
            let error = error.to_compile_error();
            return quote! { #input #error }.into();
        }
    };

    match hook_rules::check_impl(&input, &config) {
        Ok(()) => quote! { #input }.into(),
        Err(error) => {
            let error = error.to_compile_error();
            quote! { #input #error }.into()
        }
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
