use std::collections::HashMap;
use std::sync::Arc;

use crate::views::view::View;

/// Factory that produces a fresh root view for an app on each connection.
pub type AppFactory = Arc<dyn Fn() -> Box<dyn View> + Send + Sync>;

/// Reserved app ids, mirroring Ivy-Framework's `AppIds`.
///
/// Only the two ids that are meaningful in this framework are ported; Ivy's `$auth`
/// and `$chrome` have no counterpart here (no auth, no app shell).
pub struct AppIds;

impl AppIds {
    /// The app served when a connection supplies no app id.
    pub const DEFAULT: &'static str = "$default";
    /// The app served when a requested id cannot be resolved.
    pub const ERROR_NOT_FOUND: &'static str = "$error-not-found";
}

/// Describes a single app: its id, display title, and root view factory.
///
/// Ported from Ivy-Framework's `AppDescriptor`, reduced to the fields this framework
/// can act on. No visibility, ordering, grouping, or menu metadata.
#[derive(Clone)]
pub struct AppDescriptor {
    pub id: String,
    pub title: String,
    pub factory: AppFactory,
}

impl AppDescriptor {
    pub fn new(id: impl Into<String>, title: impl Into<String>, factory: AppFactory) -> Self {
        AppDescriptor {
            id: id.into(),
            title: title.into(),
            factory,
        }
    }

    /// Build a fresh root view for this app.
    pub fn create_view(&self) -> Box<dyn View> {
        (self.factory)()
    }
}

/// An ordered registry of apps with a default, resolved by id at connection time.
///
/// Ported from Ivy-Framework's `AppRepository` + `AppRouter` pair, collapsed into one
/// type since this framework has no reload observables or menu tree to separate.
#[derive(Default)]
pub struct AppRegistry {
    apps: HashMap<String, AppDescriptor>,
    /// Registration order, so `ids()` is stable and the first app can become the default.
    order: Vec<String>,
    default_id: Option<String>,
}

impl AppRegistry {
    pub fn new() -> Self {
        AppRegistry {
            apps: HashMap::new(),
            order: Vec::new(),
            default_id: None,
        }
    }

    /// Register an app. The first registered app becomes the default unless an app is
    /// registered under [`AppIds::DEFAULT`], which always wins. Re-registering an id
    /// replaces the descriptor and keeps its original position.
    pub fn register(
        &mut self,
        id: impl Into<String>,
        title: impl Into<String>,
        factory: AppFactory,
    ) {
        let id = id.into();
        let descriptor = AppDescriptor::new(id.clone(), title, factory);

        if !self.apps.contains_key(&id) {
            self.order.push(id.clone());
        }
        self.apps.insert(id.clone(), descriptor);

        let is_reserved_default = id == AppIds::DEFAULT;
        if is_reserved_default || self.default_id.is_none() {
            self.default_id = Some(id);
        }
    }

    /// Look up an app by exact id.
    pub fn get(&self, id: &str) -> Option<&AppDescriptor> {
        self.apps.get(id)
    }

    /// Resolve the app to serve for an optional requested id.
    ///
    /// Falls back to the default app when `app_id` is `None` or names an unknown app —
    /// the right behaviour for an initial connection, where a bookmarked dead link should
    /// still land somewhere. An explicit `Navigate` to an unknown id must use [`Self::get`]
    /// instead, so a typo does not look like a successful navigation.
    pub fn resolve(&self, app_id: Option<&str>) -> Option<&AppDescriptor> {
        if let Some(id) = app_id {
            if let Some(descriptor) = self.apps.get(id) {
                return Some(descriptor);
            }
        }
        self.default_id.as_ref().and_then(|id| self.apps.get(id))
    }

    /// The id of the default app, if any app is registered.
    pub fn default_id(&self) -> Option<&str> {
        self.default_id.as_deref()
    }

    /// All registered app ids in registration order.
    pub fn ids(&self) -> Vec<&str> {
        self.order.iter().map(|s| s.as_str()).collect()
    }

    /// Number of registered apps.
    pub fn len(&self) -> usize {
        self.apps.len()
    }

    /// Whether no apps are registered.
    pub fn is_empty(&self) -> bool {
        self.apps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::view::{BuildContext, Element};
    use crate::widgets::text::TextBlock;

    struct LabelView {
        label: String,
    }

    impl View for LabelView {
        fn build(&self, _ctx: &mut BuildContext) -> Element {
            Element::Widget(Box::new(TextBlock::new(&self.label)))
        }
    }

    fn label_factory(label: &'static str) -> AppFactory {
        Arc::new(move || {
            Box::new(LabelView {
                label: label.to_string(),
            })
        })
    }

    fn build_json(descriptor: &AppDescriptor) -> String {
        let view = descriptor.create_view();
        let mut store = crate::hooks::hook_store::HookStore::new();
        let mut ctx = BuildContext::new(&mut store, None);
        let element = view.build(&mut ctx);
        serde_json::to_value(&element).unwrap().to_string()
    }

    #[test]
    fn test_resolve_known_id() {
        let mut registry = AppRegistry::new();
        registry.register("alpha", "Alpha", label_factory("alpha-view"));
        registry.register("beta", "Beta", label_factory("beta-view"));

        let descriptor = registry.resolve(Some("beta")).expect("beta should resolve");
        assert_eq!(descriptor.id, "beta");
        assert_eq!(descriptor.title, "Beta");
        assert!(build_json(descriptor).contains("beta-view"));
    }

    #[test]
    fn test_resolve_none_returns_default() {
        let mut registry = AppRegistry::new();
        registry.register("alpha", "Alpha", label_factory("alpha-view"));
        registry.register("beta", "Beta", label_factory("beta-view"));

        // First registered app is the default.
        let descriptor = registry.resolve(None).expect("default should resolve");
        assert_eq!(descriptor.id, "alpha");
        assert_eq!(registry.default_id(), Some("alpha"));
    }

    #[test]
    fn test_resolve_unknown_id_falls_back_to_default() {
        let mut registry = AppRegistry::new();
        registry.register("alpha", "Alpha", label_factory("alpha-view"));

        let descriptor = registry
            .resolve(Some("does-not-exist"))
            .expect("should fall back to default");
        assert_eq!(descriptor.id, "alpha");
    }

    #[test]
    fn test_resolve_on_empty_registry_returns_none() {
        let registry = AppRegistry::new();
        assert!(registry.resolve(None).is_none());
        assert!(registry.resolve(Some("alpha")).is_none());
        assert!(registry.is_empty());
    }

    #[test]
    fn test_each_resolve_yields_independent_view_instance() {
        let mut registry = AppRegistry::new();
        registry.register("alpha", "Alpha", label_factory("alpha-view"));

        let descriptor = registry.resolve(Some("alpha")).unwrap();
        let first = descriptor.create_view();
        let second = descriptor.create_view();

        // Two separate boxed views, not a shared instance.
        assert!(!std::ptr::eq(
            first.as_ref() as *const dyn View as *const u8,
            second.as_ref() as *const dyn View as *const u8
        ));
    }

    #[test]
    fn test_reserved_default_id_wins() {
        let mut registry = AppRegistry::new();
        registry.register("alpha", "Alpha", label_factory("alpha-view"));
        registry.register(AppIds::DEFAULT, "Root", label_factory("root-view"));

        assert_eq!(registry.default_id(), Some(AppIds::DEFAULT));
        let descriptor = registry.resolve(None).unwrap();
        assert!(build_json(descriptor).contains("root-view"));
    }

    #[test]
    fn test_get_requires_exact_id() {
        let mut registry = AppRegistry::new();
        registry.register("alpha", "Alpha", label_factory("alpha-view"));

        assert!(registry.get("alpha").is_some());
        // Unlike resolve, get does NOT fall back to the default.
        assert!(registry.get("does-not-exist").is_none());
    }

    #[test]
    fn test_ids_preserve_registration_order() {
        let mut registry = AppRegistry::new();
        registry.register("charlie", "Charlie", label_factory("c"));
        registry.register("alpha", "Alpha", label_factory("a"));
        registry.register("bravo", "Bravo", label_factory("b"));

        assert_eq!(registry.ids(), vec!["charlie", "alpha", "bravo"]);
    }

    #[test]
    fn test_re_register_replaces_and_keeps_position() {
        let mut registry = AppRegistry::new();
        registry.register("alpha", "Alpha", label_factory("first"));
        registry.register("beta", "Beta", label_factory("beta-view"));
        registry.register("alpha", "Alpha v2", label_factory("second"));

        assert_eq!(registry.len(), 2);
        assert_eq!(registry.ids(), vec!["alpha", "beta"]);
        let descriptor = registry.get("alpha").unwrap();
        assert_eq!(descriptor.title, "Alpha v2");
        assert!(build_json(descriptor).contains("second"));
    }

    #[test]
    fn test_reserved_ids() {
        assert_eq!(AppIds::DEFAULT, "$default");
        assert_eq!(AppIds::ERROR_NOT_FOUND, "$error-not-found");
    }
}
