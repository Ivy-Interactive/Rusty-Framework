use crate::core::event_registry::EventRegistry;
use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// A container for `Field`-wrapped inputs with a submit action.
#[derive(Clone, Serialize, Deserialize)]
pub struct Form {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub children: Vec<Element>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submit_title: Option<String>,
    pub disabled: bool,
    #[serde(skip)]
    pub on_submit: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Form")
            .field("children", &self.children.len())
            .field("submit_title", &self.submit_title)
            .field("disabled", &self.disabled)
            .finish()
    }
}

impl Form {
    pub fn new() -> Self {
        Form {
            id: None,
            children: Vec::new(),
            submit_title: None,
            disabled: false,
            on_submit: None,
        }
    }

    pub fn child(mut self, element: impl Into<Element>) -> Self {
        self.children.push(element.into());
        self
    }

    pub fn children(mut self, elements: Vec<Element>) -> Self {
        self.children.extend(elements);
        self
    }

    pub fn submit_title(mut self, title: &str) -> Self {
        self.submit_title = Some(title.to_string());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_submit(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_submit = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for Form {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetData for Form {
    fn widget_type(&self) -> &str {
        "form"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "form",
            "id": self.id,
            "children": self.children.iter()
                .map(|c| serde_json::to_value(c).unwrap_or_default())
                .collect::<Vec<_>>(),
            "submitTitle": self.submit_title,
            "disabled": self.disabled,
            "hasOnSubmit": self.on_submit.is_some(),
        })
    }

    fn clone_box(&self) -> Box<dyn WidgetData> {
        Box::new(self.clone())
    }

    fn assign_id(&mut self, id: String) {
        self.id = Some(id);
    }

    fn get_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    fn register_events(&self, widget_id: &str, registry: &mut EventRegistry) {
        if let Some(handler) = &self.on_submit {
            let handler = handler.clone();
            registry.register(widget_id, "submit", Arc::new(move |_args| handler()));
        }
    }

    fn children_mut(&mut self) -> Option<&mut Vec<Element>> {
        Some(&mut self.children)
    }
}

impl From<Form> for Element {
    fn from(form: Form) -> Self {
        form.into_element()
    }
}

/// A label / description / error wrapper around a single input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid: Option<String>,
    pub child: Box<Element>,
}

impl Field {
    pub fn new(label: &str, child: impl Into<Element>) -> Self {
        Field {
            id: None,
            label: label.to_string(),
            description: None,
            help: None,
            required: false,
            invalid: None,
            child: Box::new(child.into()),
        }
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn help(mut self, help: &str) -> Self {
        self.help = Some(help.to_string());
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn invalid(mut self, message: &str) -> Self {
        self.invalid = Some(message.to_string());
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for Field {
    fn widget_type(&self) -> &str {
        "field"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "field",
            "id": self.id,
            "label": self.label,
            "description": self.description,
            "help": self.help,
            "required": self.required,
            "invalid": self.invalid,
            "child": serde_json::to_value(&*self.child).unwrap_or_default(),
        })
    }

    fn clone_box(&self) -> Box<dyn WidgetData> {
        Box::new(self.clone())
    }

    fn assign_id(&mut self, id: String) {
        self.id = Some(id);
    }

    fn get_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    fn single_child_mut(&mut self) -> Option<&mut Element> {
        Some(&mut self.child)
    }
}

impl From<Field> for Element {
    fn from(field: Field) -> Self {
        field.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::BuildContext;
    use crate::widgets::input::TextInput;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_form_builder() {
        let form = Form::new()
            .child(Field::new("Name", TextInput::new()))
            .submit_title("Save")
            .disabled(true);

        assert_eq!(form.children.len(), 1);
        assert_eq!(form.submit_title.as_deref(), Some("Save"));
        assert!(form.disabled);
    }

    #[test]
    fn test_form_to_json() {
        let json = Form::new()
            .child(Field::new("Name", TextInput::new()))
            .submit_title("Save")
            .on_submit(|| {})
            .to_json();

        assert_eq!(json["type"], "form");
        assert_eq!(json["submitTitle"], "Save");
        assert_eq!(json["disabled"], false);
        assert_eq!(json["hasOnSubmit"], true);
        assert_eq!(json["children"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_field_to_json() {
        let json = Field::new("Email", TextInput::new())
            .description("We never share it")
            .help("Work address preferred")
            .required(true)
            .invalid("Required")
            .to_json();

        assert_eq!(json["type"], "field");
        assert_eq!(json["label"], "Email");
        assert_eq!(json["description"], "We never share it");
        assert_eq!(json["help"], "Work address preferred");
        assert_eq!(json["required"], true);
        assert_eq!(json["invalid"], "Required");
        assert_eq!(json["child"]["type"], "text_input");
    }

    #[test]
    fn test_form_children_mut_recursion_assigns_ids() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = Form::new()
            .child(Field::new("Name", TextInput::new()))
            .child(Field::new("Email", TextInput::new()))
            .into();
        element.assign_ids(&mut ctx);

        // Form w-0, Field w-1, its input w-2, Field w-3, its input w-4
        if let Element::Widget(ref form) = element {
            assert_eq!(form.get_id(), Some("w-0"));
            let json = form.to_json();
            assert_eq!(json["children"][0]["id"], "w-1");
            assert_eq!(json["children"][0]["child"]["id"], "w-2");
            assert_eq!(json["children"][1]["id"], "w-3");
            assert_eq!(json["children"][1]["child"]["id"], "w-4");
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_field_single_child_mut_recursion() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = Field::new("Name", TextInput::new()).into();
        element.assign_ids(&mut ctx);

        if let Element::Widget(ref mut field) = element {
            assert_eq!(field.get_id(), Some("w-0"));
            if let Some(Element::Widget(child)) = field.single_child_mut() {
                assert_eq!(child.get_id(), Some("w-1"));
            } else {
                panic!("Expected Field child to be a Widget");
            }
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_form_submit_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let submitted = Arc::new(AtomicBool::new(false));
        let submitted_clone = submitted.clone();

        let mut element: Element = Form::new()
            .on_submit(move || {
                submitted_clone.store(true, Ordering::SeqCst);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "submit", serde_json::Value::Null));
        assert!(submitted.load(Ordering::SeqCst));
    }

    #[test]
    fn test_form_nested_input_change_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received = Arc::new(std::sync::Mutex::new(String::new()));
        let received_clone = received.clone();

        let mut element: Element = Form::new()
            .child(Field::new(
                "Name",
                TextInput::new().on_change(move |v| {
                    *received_clone.lock().unwrap() = v;
                }),
            ))
            .into();
        element.assign_ids(&mut ctx);

        // Form w-0, Field w-1, TextInput w-2
        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-2", "change", json!({"value": "Ada"})));
        assert_eq!(*received.lock().unwrap(), "Ada");
    }

    #[test]
    fn test_form_into_element() {
        let el: Element = Form::new().into();
        assert!(matches!(el, Element::Widget(_)));
    }
}
