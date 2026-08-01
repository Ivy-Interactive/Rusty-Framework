use std::sync::Arc;

use crate::hooks::use_ref::use_ref;
use crate::hooks::use_state::{use_state, State};
use crate::views::view::{BuildContext, Element};

/// Render an element on demand, carrying a value from the trigger to the factory.
///
/// Ported from Ivy-Framework's `Hooks/UseTrigger.cs`. Returns the element to
/// render — [`Element::Empty`] until triggered — plus the function that triggers
/// it with a value. The classic use is a sheet or dialog that needs to know which
/// row the user clicked.
///
/// `factory` receives the open `State<bool>` so the element can close itself, the
/// way Ivy hands its `IState<bool>` to the factory.
///
/// Ivy nests the element in a `FuncView` so triggering does not re-render the
/// parent. Rusty's `child_view` needs a `HookStore` the caller must thread, which
/// a hook cannot do, so the state lives in the calling view and the parent
/// rebuild that `State::set` already triggers re-renders the element.
pub fn use_trigger<T, F>(
    ctx: &mut BuildContext,
    factory: F,
) -> (Element, Arc<dyn Fn(T) + Send + Sync>)
where
    T: Send + Sync + Clone + 'static,
    F: Fn(State<bool>, T) -> Element,
{
    let open = use_state(ctx, false);
    // The value is set together with `open`, so it needs no rebuild of its own.
    let value = use_ref(ctx, None::<T>);

    let element = match (open.get(), value.get()) {
        (true, Some(value)) => factory(open.clone(), value),
        _ => Element::Empty,
    };

    let trigger: Arc<dyn Fn(T) + Send + Sync> = {
        let open = open.clone();
        let value = value.clone();
        Arc::new(move |triggered| {
            value.set(Some(triggered));
            open.set(true);
        })
    };

    (element, trigger)
}

/// Render an element on demand, with nothing to carry.
///
/// The overload of Ivy's `UseTrigger` that takes no value.
pub fn use_trigger_unit<F>(
    ctx: &mut BuildContext,
    factory: F,
) -> (Element, Arc<dyn Fn() + Send + Sync>)
where
    F: Fn(State<bool>) -> Element,
{
    let open = use_state(ctx, false);

    let element = if open.get() {
        factory(open.clone())
    } else {
        Element::Empty
    };

    let trigger: Arc<dyn Fn() + Send + Sync> = {
        let open = open.clone();
        Arc::new(move || open.set(true))
    };

    (element, trigger)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::widgets::text::TextBlock;

    /// A factory rendering its value, so a test can read it back out of the tree.
    fn label_factory(_open: State<bool>, value: String) -> Element {
        TextBlock::new(&value).into_element()
    }

    fn rendered_text(element: &Element) -> String {
        let Element::Widget(widget) = element else {
            panic!("expected a widget, got {element:?}");
        };
        widget.to_json()["content"]
            .as_str()
            .expect("a text block has content")
            .to_string()
    }

    #[test]
    fn test_nothing_renders_before_the_trigger() {
        let mut store = HookStore::new();
        let mut ctx = BuildContext::new(&mut store, None);

        let (element, _trigger) = use_trigger(&mut ctx, label_factory);
        assert!(matches!(element, Element::Empty));
    }

    #[test]
    fn test_triggered_value_reaches_the_factory() {
        let mut store = HookStore::new();

        let trigger = {
            let mut ctx = BuildContext::new(&mut store, None);
            use_trigger(&mut ctx, label_factory).1
        };
        trigger("row-42".to_string());

        let mut ctx = BuildContext::new(&mut store, None);
        let (element, _trigger) = use_trigger(&mut ctx, label_factory);
        assert_eq!(rendered_text(&element), "row-42");

        // Triggering again with a different value re-renders with the new one.
        let trigger = {
            let mut ctx = BuildContext::new(&mut store, None);
            use_trigger(&mut ctx, label_factory).1
        };
        trigger("row-7".to_string());
        let mut ctx = BuildContext::new(&mut store, None);
        let (element, _trigger) = use_trigger(&mut ctx, label_factory);
        assert_eq!(rendered_text(&element), "row-7");
    }

    #[test]
    fn test_the_factorys_open_state_closes_it() {
        let mut store = HookStore::new();

        let trigger = {
            let mut ctx = BuildContext::new(&mut store, None);
            use_trigger(&mut ctx, label_factory).1
        };
        trigger("visible".to_string());

        // The factory closes the element through the `State<bool>` it was handed.
        {
            let mut ctx = BuildContext::new(&mut store, None);
            let (element, _trigger) = use_trigger(&mut ctx, |open, value: String| {
                open.set(false);
                TextBlock::new(&value).into_element()
            });
            assert_eq!(rendered_text(&element), "visible");
        }

        let mut ctx = BuildContext::new(&mut store, None);
        let (element, _trigger) = use_trigger(&mut ctx, label_factory);
        assert!(
            matches!(element, Element::Empty),
            "setting open to false must hide the element"
        );
    }

    #[test]
    fn test_unit_trigger_opens_and_closes() {
        let mut store = HookStore::new();

        let trigger = {
            let mut ctx = BuildContext::new(&mut store, None);
            let (element, trigger) =
                use_trigger_unit(&mut ctx, |_open| TextBlock::new("sheet").into_element());
            assert!(matches!(element, Element::Empty));
            trigger
        };
        trigger();

        {
            let mut ctx = BuildContext::new(&mut store, None);
            let (element, _trigger) = use_trigger_unit(&mut ctx, |open| {
                open.set(false);
                TextBlock::new("sheet").into_element()
            });
            assert_eq!(rendered_text(&element), "sheet");
        }

        let mut ctx = BuildContext::new(&mut store, None);
        let (element, _trigger) =
            use_trigger_unit(&mut ctx, |_open| TextBlock::new("sheet").into_element());
        assert!(matches!(element, Element::Empty));
    }
}
