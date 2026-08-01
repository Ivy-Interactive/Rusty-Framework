//! Declarative form construction over an arbitrary model type.
//!
//! Ivy's `FormBuilder` binds fields with expression trees and reflection. Rust has
//! neither, so a field is registered with an explicit **render closure** that receives
//! the current model plus a setter that replaces it. There is no auto-scaffolding
//! from the model type, and fields render in registration order — use
//! [`crate::widgets::Layout`] if you need column layout.

use std::collections::HashMap;
use std::sync::Arc;

use crate::views::view::Element;
use crate::widgets::form::{Field, Form};

/// Replaces the bound model with a new value, triggering a rebuild.
pub type ModelSetter<M> = Arc<dyn Fn(M) + Send + Sync>;

/// Renders one field: receives the current model and a setter that replaces it.
pub type FieldRender<M> = Arc<dyn Fn(&M, ModelSetter<M>) -> Element + Send + Sync>;

/// Validates a model, returning `Err(message)` for the field it is registered on.
pub type Validator<M> = Arc<dyn Fn(&M) -> Result<(), String> + Send + Sync>;

/// Receives the validated model when the form is submitted.
pub type SubmitHandler<M> = Arc<dyn Fn(&M) + Send + Sync>;

/// A single registered field of a [`FormBuilder`].
struct FieldSpec<M> {
    name: String,
    label: String,
    render: FieldRender<M>,
    description: Option<String>,
    help: Option<String>,
    required: bool,
    validators: Vec<Validator<M>>,
}

impl<M> std::fmt::Debug for FieldSpec<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldSpec")
            .field("name", &self.name)
            .field("label", &self.label)
            .field("required", &self.required)
            .field("validators", &self.validators.len())
            .finish()
    }
}

/// Builds a [`Form`] of [`Field`]-wrapped inputs bound to a model of type `M`.
pub struct FormBuilder<M: Clone + Send + Sync + 'static> {
    fields: Vec<FieldSpec<M>>,
    on_submit: Option<SubmitHandler<M>>,
    submit_title: Option<String>,
    disabled: bool,
}

impl<M: Clone + Send + Sync + 'static> std::fmt::Debug for FormBuilder<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormBuilder")
            .field("fields", &self.fields)
            .field("submit_title", &self.submit_title)
            .field("disabled", &self.disabled)
            .field("has_on_submit", &self.on_submit.is_some())
            .finish()
    }
}

impl<M: Clone + Send + Sync + 'static> Clone for FormBuilder<M> {
    fn clone(&self) -> Self {
        FormBuilder {
            fields: self
                .fields
                .iter()
                .map(|f| FieldSpec {
                    name: f.name.clone(),
                    label: f.label.clone(),
                    render: f.render.clone(),
                    description: f.description.clone(),
                    help: f.help.clone(),
                    required: f.required,
                    validators: f.validators.clone(),
                })
                .collect(),
            on_submit: self.on_submit.clone(),
            submit_title: self.submit_title.clone(),
            disabled: self.disabled,
        }
    }
}

impl<M: Clone + Send + Sync + 'static> FormBuilder<M> {
    pub fn new() -> Self {
        FormBuilder {
            fields: Vec::new(),
            on_submit: None,
            submit_title: None,
            disabled: false,
        }
    }

    /// Register one field. `render` receives the current model and a setter
    /// that replaces it — the Rust stand-in for Ivy's expression-tree binding.
    pub fn field(mut self, name: &str, label: &str, render: FieldRender<M>) -> Self {
        self.fields.push(FieldSpec {
            name: name.to_string(),
            label: label.to_string(),
            render,
            description: None,
            help: None,
            required: false,
            validators: Vec::new(),
        });
        self
    }

    /// Set a field's description. A no-op when `name` is not registered.
    pub fn description(mut self, name: &str, text: &str) -> Self {
        if let Some(field) = self.find_mut(name) {
            field.description = Some(text.to_string());
        }
        self
    }

    /// Set a field's help text. A no-op when `name` is not registered.
    pub fn help(mut self, name: &str, text: &str) -> Self {
        if let Some(field) = self.find_mut(name) {
            field.help = Some(text.to_string());
        }
        self
    }

    /// Mark a field required. A no-op when `name` is not registered.
    pub fn required(mut self, name: &str) -> Self {
        if let Some(field) = self.find_mut(name) {
            field.required = true;
        }
        self
    }

    /// Attach a validator to a field. A no-op when `name` is not registered.
    pub fn validate(mut self, name: &str, validator: Validator<M>) -> Self {
        if let Some(field) = self.find_mut(name) {
            field.validators.push(validator);
        }
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn submit_title(mut self, title: &str) -> Self {
        self.submit_title = Some(title.to_string());
        self
    }

    pub fn on_submit(mut self, handler: impl Fn(&M) + Send + Sync + 'static) -> Self {
        self.on_submit = Some(Arc::new(handler));
        self
    }

    /// The registered field names, in registration order.
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.iter().map(|f| f.name.as_str()).collect()
    }

    /// The submit handler, if one was registered.
    pub fn submit_handler(&self) -> Option<SubmitHandler<M>> {
        self.on_submit.clone()
    }

    /// Validate every field, returning per-field errors keyed by field name.
    /// The first failing validator of a field wins.
    pub fn validate_all(&self, model: &M) -> HashMap<String, String> {
        let mut errors = HashMap::new();
        for field in &self.fields {
            for validator in &field.validators {
                if let Err(message) = validator(model) {
                    errors.insert(field.name.clone(), message);
                    break;
                }
            }
        }
        errors
    }

    /// Render the model into a [`Form`] of [`Field`]-wrapped inputs, showing `errors`.
    pub fn build_form(
        &self,
        model: &M,
        errors: &HashMap<String, String>,
        set_model: ModelSetter<M>,
    ) -> Form {
        let mut form = Form::new().disabled(self.disabled);
        if let Some(title) = &self.submit_title {
            form = form.submit_title(title);
        }
        for spec in &self.fields {
            let child = (spec.render)(model, set_model.clone());
            let mut field = Field::new(&spec.label, child).required(spec.required);
            if let Some(description) = &spec.description {
                field = field.description(description);
            }
            if let Some(help) = &spec.help {
                field = field.help(help);
            }
            if let Some(message) = errors.get(&spec.name) {
                field = field.invalid(message);
            }
            form = form.child(field);
        }
        form
    }

    fn find_mut(&mut self, name: &str) -> Option<&mut FieldSpec<M>> {
        self.fields.iter_mut().find(|f| f.name == name)
    }
}

impl<M: Clone + Send + Sync + 'static> Default for FormBuilder<M> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::validators;
    use crate::views::view::WidgetData;
    use crate::widgets::input::TextInput;

    #[derive(Debug, Clone, PartialEq, Default)]
    struct Person {
        name: String,
        email: String,
    }

    fn name_field() -> FieldRender<Person> {
        Arc::new(|model: &Person, set: ModelSetter<Person>| {
            let current = model.clone();
            TextInput::new()
                .value(&model.name)
                .on_change(move |v| {
                    let mut next = current.clone();
                    next.name = v;
                    set(next);
                })
                .into()
        })
    }

    fn email_field() -> FieldRender<Person> {
        Arc::new(|model: &Person, set: ModelSetter<Person>| {
            let current = model.clone();
            TextInput::new()
                .value(&model.email)
                .on_change(move |v| {
                    let mut next = current.clone();
                    next.email = v;
                    set(next);
                })
                .into()
        })
    }

    fn noop_setter() -> ModelSetter<Person> {
        Arc::new(|_| {})
    }

    #[test]
    fn test_field_registration_order_preserved() {
        let builder = FormBuilder::<Person>::new()
            .field("name", "Name", name_field())
            .field("email", "Email", email_field());

        assert_eq!(builder.field_names(), vec!["name", "email"]);
    }

    #[test]
    fn test_validate_all_returns_one_entry_per_failing_field() {
        let builder = FormBuilder::<Person>::new()
            .field("name", "Name", name_field())
            .field("email", "Email", email_field())
            .validate(
                "name",
                Arc::new(|m: &Person| validators::not_empty(&m.name)),
            )
            .validate(
                "email",
                Arc::new(|m: &Person| validators::not_empty(&m.email)),
            )
            .validate("email", Arc::new(|m: &Person| validators::email(&m.email)));

        let errors = builder.validate_all(&Person::default());
        assert_eq!(errors.len(), 2);
        assert_eq!(errors["name"], "This field is required");
        // First failing validator of a field wins.
        assert_eq!(errors["email"], "This field is required");
    }

    #[test]
    fn test_validate_all_empty_on_success() {
        let builder = FormBuilder::<Person>::new()
            .field("name", "Name", name_field())
            .validate(
                "name",
                Arc::new(|m: &Person| validators::not_empty(&m.name)),
            )
            .validate(
                "name",
                Arc::new(|m: &Person| validators::min_length(&m.name, 2)),
            );

        let model = Person {
            name: "Ada".into(),
            email: String::new(),
        };
        assert!(builder.validate_all(&model).is_empty());
    }

    #[test]
    fn test_validate_all_reports_second_validator_failure() {
        let builder = FormBuilder::<Person>::new()
            .field("email", "Email", email_field())
            .validate(
                "email",
                Arc::new(|m: &Person| validators::not_empty(&m.email)),
            )
            .validate("email", Arc::new(|m: &Person| validators::email(&m.email)));

        let model = Person {
            name: String::new(),
            email: "nope".into(),
        };
        let errors = builder.validate_all(&model);
        assert_eq!(errors["email"], "Please enter a valid email address");
    }

    #[test]
    fn test_build_form_produces_one_field_per_registration() {
        let builder = FormBuilder::<Person>::new()
            .field("name", "Name", name_field())
            .field("email", "Email", email_field())
            .required("name")
            .description("email", "We never share it")
            .help("email", "Work address preferred")
            .submit_title("Save");

        let mut errors = HashMap::new();
        errors.insert("email".to_string(), "Invalid".to_string());

        let form = builder.build_form(&Person::default(), &errors, noop_setter());
        let json = form.to_json();

        assert_eq!(json["submitTitle"], "Save");
        let children = json["children"].as_array().unwrap();
        assert_eq!(children.len(), 2);

        assert_eq!(children[0]["type"], "field");
        assert_eq!(children[0]["label"], "Name");
        assert_eq!(children[0]["required"], true);
        assert!(children[0]["invalid"].is_null());

        assert_eq!(children[1]["label"], "Email");
        assert_eq!(children[1]["required"], false);
        assert_eq!(children[1]["invalid"], "Invalid");
        assert_eq!(children[1]["description"], "We never share it");
        assert_eq!(children[1]["help"], "Work address preferred");
    }

    #[test]
    fn test_build_form_renders_current_model_values() {
        let builder = FormBuilder::<Person>::new().field("name", "Name", name_field());
        let model = Person {
            name: "Ada".into(),
            email: String::new(),
        };
        let form = builder.build_form(&model, &HashMap::new(), noop_setter());
        let json = form.to_json();
        assert_eq!(json["children"][0]["child"]["value"], "Ada");
    }

    #[test]
    fn test_build_form_setter_updates_model() {
        let builder = FormBuilder::<Person>::new().field("name", "Name", name_field());
        let captured = Arc::new(std::sync::Mutex::new(None::<Person>));
        let captured_clone = captured.clone();
        let setter: ModelSetter<Person> = Arc::new(move |m| {
            *captured_clone.lock().unwrap() = Some(m);
        });

        let form = builder.build_form(&Person::default(), &HashMap::new(), setter);

        // Reach into the rendered TextInput and fire its on_change directly.
        let field_el = &form.children[0];
        let Element::Widget(field) = field_el else {
            panic!("Expected Field widget");
        };
        let child_json = field.to_json();
        assert_eq!(child_json["child"]["hasOnChange"], true);

        // Dispatch through the registry to exercise the real wiring.
        use crate::hooks::hook_store::HookStore;
        use crate::views::view::BuildContext;
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let mut element: Element = builder
            .build_form(&Person::default(), &HashMap::new(), {
                let captured_clone = captured.clone();
                Arc::new(move |m| {
                    *captured_clone.lock().unwrap() = Some(m);
                })
            })
            .into();
        element.assign_ids(&mut ctx);
        let registry = ctx.take_event_registry();
        // Form w-0, Field w-1, TextInput w-2
        assert!(registry.dispatch("w-2", "change", serde_json::json!({"value": "Grace"})));

        let updated = captured.lock().unwrap().clone().expect("setter not called");
        assert_eq!(updated.name, "Grace");
    }

    #[test]
    fn test_unknown_field_name_is_noop() {
        let builder = FormBuilder::<Person>::new()
            .field("name", "Name", name_field())
            .description("nope", "ignored")
            .help("nope", "ignored")
            .required("nope")
            .validate("nope", Arc::new(|_: &Person| Err("never".to_string())));

        assert_eq!(builder.field_names(), vec!["name"]);
        assert!(builder.validate_all(&Person::default()).is_empty());

        let form = builder.build_form(&Person::default(), &HashMap::new(), noop_setter());
        let json = form.to_json();
        assert_eq!(json["children"].as_array().unwrap().len(), 1);
        assert_eq!(json["children"][0]["required"], false);
        assert!(json["children"][0]["description"].is_null());
    }

    #[test]
    fn test_disabled_propagates_to_form() {
        let builder = FormBuilder::<Person>::new().disabled(true);
        let form = builder.build_form(&Person::default(), &HashMap::new(), noop_setter());
        assert_eq!(form.to_json()["disabled"], true);
    }

    #[test]
    fn test_default_builder_is_empty() {
        let builder = FormBuilder::<Person>::default();
        assert!(builder.field_names().is_empty());
        assert!(builder.submit_handler().is_none());
    }
}
