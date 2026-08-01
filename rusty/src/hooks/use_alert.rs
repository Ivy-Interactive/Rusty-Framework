use std::sync::Arc;

use crate::hooks::use_ref::use_ref;
use crate::hooks::use_state::use_state;
use crate::views::alerts::{AlertOptions, AlertResult};
use crate::views::view::{BuildContext, Element};
use crate::widgets::button::Button;
use crate::widgets::dialog::Dialog;
use crate::widgets::text::TextBlock;

/// The callback an alert reports its result to.
pub type AlertCallback = Arc<dyn Fn(AlertResult) + Send + Sync>;

/// Opens an alert: `(message, title, button_set, callback)`.
///
/// Ivy's `ShowAlertDelegate` uses optional and defaulted parameters, which Rust
/// has no equivalent for, so every argument is explicit here.
pub type ShowAlert = Arc<
    dyn Fn(&str, Option<&str>, crate::views::alerts::AlertButtonSet, AlertCallback) + Send + Sync,
>;

/// Show a modal alert from anywhere in a view, and get its result in a callback.
///
/// Ported from Ivy-Framework's `Views/Alerts/UseAlert.cs`. Returns the element to
/// render — [`Element::Empty`] while closed — plus the function that opens it.
/// Put the element anywhere in the returned tree; it renders nothing until shown.
///
/// Ivy nests the dialog in a `FuncView` so opening it does not re-render the
/// parent. Rusty's `child_view` needs a `HookStore` the caller must thread, which
/// a hook cannot do, so the state lives in the calling view and the parent
/// rebuild that `State::set` already triggers re-renders the dialog.
pub fn use_alert(ctx: &mut BuildContext) -> (Element, ShowAlert) {
    // Only `open` triggers the rebuild; the options and callback are set in the
    // same breath, so making them refs avoids two redundant rebuild signals.
    let open = use_state(ctx, false);
    let options = use_ref(ctx, None::<AlertOptions>);
    let callback = use_ref(ctx, None::<AlertCallback>);

    let element = match (open.get(), options.get()) {
        (true, Some(options)) => {
            let buttons = options
                .buttons
                .iter()
                .map(|button| {
                    let result = button.result;
                    let open = open.clone();
                    let callback = callback.clone();
                    Button::new(&button.label)
                        .variant(button.variant)
                        .on_click(move || {
                            if let Some(callback) = callback.get() {
                                callback(result);
                            }
                            open.set(false);
                        })
                        .into_element()
                })
                .collect::<Vec<_>>();

            let mut dialog = Dialog::new(true);
            if let Some(title) = &options.title {
                dialog = dialog.title(title);
            }
            dialog
                .child(TextBlock::new(options.message.as_deref().unwrap_or("")))
                .footer(buttons)
                .into_element()
        }
        _ => Element::Empty,
    };

    let show_alert: ShowAlert = {
        let open = open.clone();
        let options = options.clone();
        let callback = callback.clone();
        Arc::new(move |message, title, button_set, on_result| {
            options.set(Some(AlertOptions::new(
                title.map(str::to_string),
                Some(message.to_string()),
                button_set,
            )));
            callback.set(Some(on_result));
            open.set(true);
        })
    };

    (element, show_alert)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::event_registry::EventRegistry;
    use crate::hooks::hook_store::HookStore;
    use crate::views::alerts::AlertButtonSet;
    use crate::widgets::button::ButtonVariant;
    use std::sync::Mutex;

    /// The label and variant of each footer button, read off the serialized tree.
    fn footer_buttons(element: &Element) -> Vec<(String, ButtonVariant)> {
        let Element::Widget(widget) = element else {
            panic!("expected a dialog widget, got {element:?}");
        };
        widget.to_json()["footer"]
            .as_array()
            .expect("an alert dialog always has a footer")
            .iter()
            .map(|button| {
                let title = button["title"].as_str().expect("a button has a title");
                let variant: ButtonVariant = serde_json::from_value(button["variant"].clone())
                    .expect("a button variant round-trips");
                (title.to_string(), variant)
            })
            .collect()
    }

    /// Build the alert, assign widget ids, and return the tree with its handlers,
    /// which is how the runtime makes a button clickable.
    fn build_alert(store: &mut HookStore) -> (Element, EventRegistry) {
        let mut ctx = BuildContext::new(store, None);
        let (mut element, _show_alert) = use_alert(&mut ctx);
        element.assign_ids(&mut ctx);
        (element, ctx.take_event_registry())
    }

    #[test]
    fn test_closed_alert_renders_nothing() {
        let mut store = HookStore::new();
        let mut ctx = BuildContext::new(&mut store, None);

        let (element, _show_alert) = use_alert(&mut ctx);
        assert!(matches!(element, Element::Empty));
    }

    #[test]
    fn test_show_alert_opens_a_dialog_with_the_title_and_message() {
        let mut store = HookStore::new();

        let show_alert = {
            let mut ctx = BuildContext::new(&mut store, None);
            let (_element, show_alert) = use_alert(&mut ctx);
            show_alert
        };
        show_alert(
            "Really delete?",
            Some("Delete file"),
            AlertButtonSet::OkCancel,
            Arc::new(|_| {}),
        );

        // The rebuild `open.set(true)` asked for.
        let mut ctx = BuildContext::new(&mut store, None);
        let (element, _show_alert) = use_alert(&mut ctx);

        let Element::Widget(widget) = &element else {
            panic!("expected a dialog, got {element:?}");
        };
        let json = widget.to_json();
        assert_eq!(json["type"], "dialog");
        assert_eq!(json["open"], true);
        assert_eq!(json["title"], "Delete file");
        assert_eq!(json["children"][0]["content"], "Really delete?");
    }

    #[test]
    fn test_each_button_set_renders_ivys_labels_and_variants() {
        for (set, expected) in [
            (AlertButtonSet::Ok, vec![("Ok", ButtonVariant::Primary)]),
            (
                AlertButtonSet::OkCancel,
                vec![
                    ("Cancel", ButtonVariant::Secondary),
                    ("Ok", ButtonVariant::Primary),
                ],
            ),
            (
                AlertButtonSet::YesNo,
                vec![
                    ("No", ButtonVariant::Secondary),
                    ("Yes", ButtonVariant::Primary),
                ],
            ),
            (
                AlertButtonSet::YesNoCancel,
                vec![
                    ("Cancel", ButtonVariant::Secondary),
                    ("No", ButtonVariant::Primary),
                    ("Yes", ButtonVariant::Primary),
                ],
            ),
        ] {
            let mut store = HookStore::new();
            let show_alert = {
                let mut ctx = BuildContext::new(&mut store, None);
                use_alert(&mut ctx).1
            };
            show_alert("pick one", None, set, Arc::new(|_| {}));

            let mut ctx = BuildContext::new(&mut store, None);
            let (element, _show_alert) = use_alert(&mut ctx);
            let expected = expected
                .into_iter()
                .map(|(label, variant)| (label.to_string(), variant))
                .collect::<Vec<_>>();

            assert_eq!(footer_buttons(&element), expected, "button set {set:?}");
        }
    }

    #[test]
    fn test_clicking_a_button_reports_its_result_and_closes() {
        let mut store = HookStore::new();
        let results = Arc::new(Mutex::new(Vec::<AlertResult>::new()));

        let show_alert = {
            let mut ctx = BuildContext::new(&mut store, None);
            use_alert(&mut ctx).1
        };
        {
            let results = results.clone();
            show_alert(
                "Save changes?",
                None,
                AlertButtonSet::YesNo,
                Arc::new(move |result| results.lock().unwrap().push(result)),
            );
        }

        // Click "Yes", the second button in the set, the way the runtime does.
        let (element, registry) = build_alert(&mut store);
        let buttons = footer_buttons(&element);
        assert_eq!(buttons[1].0, "Yes");

        let Element::Widget(dialog) = &element else {
            unreachable!("just asserted it is a dialog");
        };
        let yes_id = dialog.to_json()["footer"][1]["id"]
            .as_str()
            .expect("assign_ids gives every button an id")
            .to_string();
        assert!(
            registry.dispatch(&yes_id, "click", serde_json::Value::Null),
            "the alert's buttons must be registered as click handlers"
        );

        assert_eq!(*results.lock().unwrap(), vec![AlertResult::Yes]);

        // Clicking closed it, so the next build renders nothing.
        let mut ctx = BuildContext::new(&mut store, None);
        let (element, _show_alert) = use_alert(&mut ctx);
        assert!(
            matches!(element, Element::Empty),
            "clicking a button must close the alert"
        );
    }
}
