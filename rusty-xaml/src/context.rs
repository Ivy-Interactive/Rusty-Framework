//! The runtime data a document is parsed against.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use serde_json::Value;

/// A named event handler, stored type-erased so the whole context is one type.
pub type Handler = Arc<dyn Fn() + Send + Sync>;

/// Values for `{Binding ..}` attributes and handlers for event attributes.
///
/// This is the only place runtime data enters a parse: the markup itself is
/// inert text. Built fluently:
///
/// ```
/// # use rusty_xaml::XamlContext;
/// let ctx = XamlContext::new()
///     .value("Title", "Dashboard")
///     .value("Count", 3)
///     .handler("OnIncrement", || println!("clicked"));
///
/// assert!(ctx.value_of("Title").is_some());
/// assert!(ctx.handler_of("OnIncrement").is_some());
/// ```
#[derive(Clone, Default)]
pub struct XamlContext {
    values: HashMap<String, Value>,
    handlers: HashMap<String, Handler>,
}

impl XamlContext {
    /// An empty context. `parse` is `parse_with` against this.
    pub fn new() -> Self {
        XamlContext::default()
    }

    /// Bind a value that `{Binding <name>}` resolves to.
    ///
    /// Any JSON scalar works: a string reaches a `&str` builder slot verbatim, a
    /// number reaches an `f64`/`usize` slot without a round trip through text.
    pub fn value(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.values.insert(name.into(), value.into());
        self
    }

    /// Register a handler an event attribute can name.
    ///
    /// The value of `Click="OnIncrement"` is a *name*, not an expression — there
    /// is no interpreter here — so the closure is supplied from Rust.
    pub fn handler(
        mut self,
        name: impl Into<String>,
        handler: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        self.handlers.insert(name.into(), Arc::new(handler));
        self
    }

    /// The value bound to `name`, if any.
    pub fn value_of(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }

    /// The handler registered under `name`, if any.
    ///
    /// Returns a clone of the `Arc` rather than a reference: attaching it means
    /// calling e.g. `Button::on_click`, which takes `impl Fn() + Send + Sync +
    /// 'static` by value, and `Arc<dyn Fn()>` does not itself implement `Fn`.
    /// The caller wraps the clone in `move || handler()`.
    pub fn handler_of(&self, name: &str) -> Option<Handler> {
        self.handlers.get(name).cloned()
    }
}

impl fmt::Debug for XamlContext {
    /// Hand-written because a `Handler` is not `Debug`; handler names are still
    /// worth printing, so they are listed as bare names.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut handlers: Vec<&str> = self.handlers.keys().map(String::as_str).collect();
        handlers.sort_unstable();

        f.debug_struct("XamlContext")
            .field("values", &self.values)
            .field("handlers", &handlers)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn values_accept_any_json_scalar() {
        let ctx = XamlContext::new()
            .value("Title", "Dashboard")
            .value("Count", 3)
            .value("Ready", true);

        assert_eq!(ctx.value_of("Title"), Some(&Value::from("Dashboard")));
        assert_eq!(ctx.value_of("Count"), Some(&Value::from(3)));
        assert_eq!(ctx.value_of("Ready"), Some(&Value::from(true)));
        assert_eq!(ctx.value_of("Missing"), None);
    }

    #[test]
    fn a_later_value_replaces_an_earlier_one_of_the_same_name() {
        let ctx = XamlContext::new().value("Count", 1).value("Count", 2);
        assert_eq!(ctx.value_of("Count"), Some(&Value::from(2)));
    }

    #[test]
    fn handler_round_trips_and_is_callable() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let ctx = XamlContext::new().handler("OnIncrement", move || {
            counter.fetch_add(1, Ordering::SeqCst);
        });

        let handler = ctx
            .handler_of("OnIncrement")
            .expect("handler is registered");
        handler();
        handler();

        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(ctx.handler_of("OnDecrement").is_none());
    }

    #[test]
    fn debug_lists_handler_names_without_requiring_debug_handlers() {
        let ctx = XamlContext::new()
            .value("Count", 1)
            .handler("OnClick", || {});
        let text = format!("{:?}", ctx);

        assert!(text.contains("Count"), "{}", text);
        assert!(text.contains("OnClick"), "{}", text);
    }
}
