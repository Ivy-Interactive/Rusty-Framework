//! Bind a model to a [`FormBuilder`]-described form, with validation on submit.

use std::collections::HashMap;
use std::sync::Arc;

use crate::hooks::use_state::{use_state, State};
use crate::views::form_builder::{FormBuilder, ModelSetter};
use crate::views::view::{BuildContext, Element};

/// Returns `(model, errors, form_element)`.
///
/// Submitting runs every validator registered on the builder; the builder's
/// `on_submit` fires only when `errors` is empty. Both the model and the error
/// map live in `use_state`, so hook ordering stays stable across rebuilds and
/// model updates signal a rebuild through [`State::set`].
pub fn use_form<M: Clone + Send + Sync + 'static>(
    ctx: &mut BuildContext,
    initial: M,
    builder: FormBuilder<M>,
) -> (State<M>, State<HashMap<String, String>>, Element) {
    let model = use_state(ctx, initial);
    let errors = use_state(ctx, HashMap::<String, String>::new());

    let current = model.get();
    let current_errors = errors.get();

    let set_model: ModelSetter<M> = {
        let model = model.clone();
        Arc::new(move |next| model.set(next))
    };

    let mut form = builder.build_form(&current, &current_errors, set_model);

    let submit_builder = builder.clone();
    let submit_model = model.clone();
    let submit_errors = errors.clone();
    form = form.on_submit(move || {
        let value = submit_model.get();
        let found = submit_builder.validate_all(&value);
        let is_valid = found.is_empty();
        submit_errors.set(found);
        if is_valid {
            if let Some(handler) = submit_builder.submit_handler() {
                handler(&value);
            }
        }
    });

    (model, errors, form.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::validators;
    use crate::views::view::View;
    use crate::widgets::input::TextInput;

    #[derive(Debug, Clone, PartialEq, Default)]
    struct Person {
        name: String,
        email: String,
    }

    fn person_builder() -> FormBuilder<Person> {
        FormBuilder::<Person>::new()
            .field(
                "name",
                "Name",
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
                }),
            )
            .field(
                "email",
                "Email",
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
                }),
            )
            .required("name")
            .validate(
                "name",
                Arc::new(|m: &Person| validators::not_empty(&m.name)),
            )
            .validate("email", Arc::new(|m: &Person| validators::email(&m.email)))
            .submit_title("Save")
    }

    struct PersonForm {
        submitted: Arc<std::sync::Mutex<Vec<Person>>>,
    }

    impl View for PersonForm {
        fn build(&self, ctx: &mut BuildContext) -> Element {
            let submitted = self.submitted.clone();
            let builder = person_builder().on_submit(move |m: &Person| {
                submitted.lock().unwrap().push(m.clone());
            });
            let (_model, _errors, element) = use_form(ctx, Person::default(), builder);
            element
        }
    }

    #[test]
    fn test_use_form_builds_form_with_one_field_per_registration() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let (_model, _errors, element) = use_form(&mut ctx, Person::default(), person_builder());

        let Element::Widget(form) = &element else {
            panic!("Expected Element::Widget");
        };
        let json = form.to_json();
        assert_eq!(json["type"], "form");
        assert_eq!(json["submitTitle"], "Save");
        assert_eq!(json["hasOnSubmit"], true);
        let children = json["children"].as_array().unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0]["label"], "Name");
        assert_eq!(children[0]["required"], true);
        assert_eq!(children[1]["label"], "Email");
    }

    #[test]
    fn test_use_form_uses_two_hook_slots() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        use_form(&mut ctx, Person::default(), person_builder());
        // The next hook allocated after use_form must be index 2.
        assert_eq!(ctx.next_hook_index(), 2);
    }

    #[test]
    fn test_use_form_hook_indices_stable_and_model_preserved_across_rebuilds() {
        let mut store = HookStore::default();

        // First build — mutate the model through the returned State.
        {
            let mut ctx = BuildContext::new(&mut store, None);
            ctx.reset();
            let (model, errors, _element) = use_form(&mut ctx, Person::default(), person_builder());
            assert_eq!(ctx.next_hook_index(), 2);
            model.set(Person {
                name: "Ada".into(),
                email: "ada@example.com".into(),
            });
            assert!(errors.get().is_empty());
        }

        // Second build — the same hook slots return the mutated model.
        {
            let mut ctx = BuildContext::new(&mut store, None);
            ctx.reset();
            let (model, _errors, element) = use_form(&mut ctx, Person::default(), person_builder());
            assert_eq!(ctx.next_hook_index(), 2);
            assert_eq!(model.get().name, "Ada");

            let Element::Widget(form) = &element else {
                panic!("Expected Element::Widget");
            };
            let json = form.to_json();
            assert_eq!(json["children"][0]["child"]["value"], "Ada");
            assert_eq!(json["children"][1]["child"]["value"], "ada@example.com");
        }
    }

    #[test]
    fn test_use_form_submit_with_failing_validator_populates_errors() {
        let mut store = HookStore::default();
        let submitted = Arc::new(std::sync::Mutex::new(Vec::<Person>::new()));
        let view = PersonForm {
            submitted: submitted.clone(),
        };

        let mut ctx = BuildContext::new(&mut store, None);
        ctx.reset();
        let mut element = view.build(&mut ctx);
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "submit", serde_json::Value::Null));

        // The default model has an empty name, so `not_empty` fails.
        assert!(
            submitted.lock().unwrap().is_empty(),
            "on_submit must not fire"
        );

        // Re-read the errors state from the persisted store on the next build.
        drop(registry);
        let mut ctx2 = BuildContext::new(&mut store, None);
        ctx2.reset();
        let (_model, errors, element2) = use_form(&mut ctx2, Person::default(), person_builder());
        assert_eq!(
            errors.get().get("name").map(String::as_str),
            Some("This field is required")
        );

        let Element::Widget(form) = &element2 else {
            panic!("Expected Element::Widget");
        };
        assert_eq!(
            form.to_json()["children"][0]["invalid"],
            "This field is required"
        );
    }

    #[test]
    fn test_use_form_submit_with_valid_model_calls_on_submit_and_clears_errors() {
        let mut store = HookStore::default();
        let submitted = Arc::new(std::sync::Mutex::new(Vec::<Person>::new()));

        // Seed the model with a valid value on the first build.
        {
            let mut ctx = BuildContext::new(&mut store, None);
            ctx.reset();
            let (model, _errors, _element) =
                use_form(&mut ctx, Person::default(), person_builder());
            model.set(Person {
                name: "Ada".into(),
                email: "ada@example.com".into(),
            });
        }

        let view = PersonForm {
            submitted: submitted.clone(),
        };
        let mut ctx = BuildContext::new(&mut store, None);
        ctx.reset();
        let mut element = view.build(&mut ctx);
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "submit", serde_json::Value::Null));

        let calls = submitted.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "Ada");
    }

    #[test]
    fn test_use_form_input_change_updates_model_state() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        ctx.reset();
        let (model, _errors, mut element) = use_form(&mut ctx, Person::default(), person_builder());
        element.assign_ids(&mut ctx);

        // Form w-0, Field w-1, name TextInput w-2
        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-2", "change", serde_json::json!({"value": "Grace"})));
        assert_eq!(model.get().name, "Grace");
    }
}
