//! The declarative menu model and the id → action binding.
//!
//! This module deliberately has no GUI dependencies. `muda::MenuEvent::send` is
//! `pub(crate)`, so a test cannot synthesize a menu click; the only way to cover the
//! binding is to make it a pure `&str` → [`MenuAction`] function and keep the muda
//! wiring in [`crate::shell`] a thin, uncovered shim.

use rusty::prelude::AppRegistry;

/// Menu item ids. Constants rather than literals so the tests can assert that every
/// declared id resolves through [`action_for_id`].
pub const ID_RELOAD: &str = "app.reload";
pub const ID_TOGGLE_DEVTOOLS: &str = "app.devtools";
pub const ID_QUIT: &str = "app.quit";

/// Prefix for the per-app navigation items built from an [`AppRegistry`].
pub const NAV_ID_PREFIX: &str = "nav.";

/// What the shell should do when a menu item is clicked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    /// Reload the WebView from the embedded server.
    Reload,
    /// Show or hide the WebView inspector.
    ToggleDevTools,
    /// Point the WebView at the given app id.
    NavigateTo(String),
    /// Exit the event loop.
    Quit,
}

/// One row of a submenu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntry {
    /// A clickable item. `id` is what the platform reports back on click.
    Item {
        id: String,
        label: String,
        action: MenuAction,
    },
    /// A horizontal rule.
    Separator,
    /// The platform's predefined quit item, which the OS labels and keys itself.
    Quit,
}

impl MenuEntry {
    /// Convenience constructor for [`MenuEntry::Item`].
    pub fn item(id: impl Into<String>, label: impl Into<String>, action: MenuAction) -> Self {
        MenuEntry::Item {
            id: id.into(),
            label: label.into(),
            action,
        }
    }

    /// The item's id, or `None` for entries the platform owns.
    pub fn id(&self) -> Option<&str> {
        match self {
            MenuEntry::Item { id, .. } => Some(id.as_str()),
            MenuEntry::Separator | MenuEntry::Quit => None,
        }
    }
}

/// A declarative description of the menu bar, walked by [`crate::shell::run`].
///
/// Building the tree as data instead of as `muda` calls is what keeps the interesting
/// part of the menu testable on a machine with no display.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuSpec {
    pub submenus: Vec<(String, Vec<MenuEntry>)>,
}

impl MenuSpec {
    pub fn new() -> Self {
        MenuSpec::default()
    }

    /// Append a submenu.
    pub fn submenu(mut self, label: impl Into<String>, entries: Vec<MenuEntry>) -> Self {
        self.submenus.push((label.into(), entries));
        self
    }

    /// Every entry across every submenu, in menu-bar order.
    pub fn entries(&self) -> impl Iterator<Item = &MenuEntry> {
        self.submenus.iter().flat_map(|(_, entries)| entries.iter())
    }

    /// Every id the platform can report back, in menu-bar order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.entries().filter_map(MenuEntry::id)
    }
}

/// The menu id for navigating to `app_id`.
pub fn nav_id(app_id: &str) -> String {
    format!("{NAV_ID_PREFIX}{app_id}")
}

/// Build the default desktop menu bar: an app menu, plus a View menu with one entry per
/// registered app.
///
/// [`AppRegistry::ids`] returns ids in registration order and [`AppRegistry::get`] yields
/// the title, which is the whole data source — the registry carries no menu metadata.
pub fn default_menu(registry: &AppRegistry) -> MenuSpec {
    let mut spec = MenuSpec::new().submenu(
        "App",
        vec![
            MenuEntry::item(ID_RELOAD, "Reload", MenuAction::Reload),
            MenuEntry::item(
                ID_TOGGLE_DEVTOOLS,
                "Toggle Developer Tools",
                MenuAction::ToggleDevTools,
            ),
            MenuEntry::Separator,
            MenuEntry::Quit,
        ],
    );

    let nav_entries: Vec<MenuEntry> = registry
        .ids()
        .into_iter()
        .map(|id| {
            // `ids()` comes from the registry's own order vector, so `get` always resolves;
            // fall back to the id as the label rather than unwrapping on framework internals.
            let label = registry
                .get(id)
                .map(|descriptor| descriptor.title.clone())
                .unwrap_or_else(|| id.to_string());
            MenuEntry::item(nav_id(id), label, MenuAction::NavigateTo(id.to_string()))
        })
        .collect();

    if !nav_entries.is_empty() {
        spec = spec.submenu("View", nav_entries);
    }

    spec
}

/// Resolve a platform-reported menu id to the action the shell should run.
///
/// Unknown ids return `None` rather than panicking: the platform can report ids the shell
/// never registered (an OS-supplied item, a stale id after a menu rebuild), and a desktop
/// app must not die on a menu click.
pub fn action_for_id(spec: &MenuSpec, id: &str) -> Option<MenuAction> {
    spec.entries().find_map(|entry| match entry {
        MenuEntry::Item {
            id: entry_id,
            action,
            ..
        } if entry_id == id => Some(action.clone()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty::prelude::{AppFactory, BuildContext, Element, TextBlock, View};
    use std::collections::HashSet;
    use std::sync::Arc;

    struct Blank;

    impl View for Blank {
        fn build(&self, _ctx: &mut BuildContext) -> Element {
            Element::Widget(Box::new(TextBlock::new("blank")))
        }
    }

    fn blank_factory() -> AppFactory {
        Arc::new(|| Box::new(Blank))
    }

    fn registry_with(apps: &[(&str, &str)]) -> AppRegistry {
        let mut registry = AppRegistry::new();
        for (id, title) in apps {
            registry.register(*id, *title, blank_factory());
        }
        registry
    }

    #[test]
    fn every_declared_id_resolves_to_an_action() {
        let spec = default_menu(&registry_with(&[
            ("reports", "Reports"),
            ("admin", "Admin"),
        ]));

        let ids: Vec<&str> = spec.ids().collect();
        assert!(!ids.is_empty(), "spec should declare at least one id");
        for id in ids {
            assert!(
                action_for_id(&spec, id).is_some(),
                "declared id {id} did not resolve"
            );
        }
    }

    #[test]
    fn unknown_ids_return_none_instead_of_panicking() {
        let spec = default_menu(&registry_with(&[("reports", "Reports")]));

        assert_eq!(action_for_id(&spec, "app.nope"), None);
        assert_eq!(action_for_id(&spec, "nav.nope"), None);
        assert_eq!(action_for_id(&spec, ""), None);
        // A prefix of a real id must not match either.
        assert_eq!(action_for_id(&spec, "app."), None);
        assert_eq!(action_for_id(&spec, "nav."), None);
    }

    #[test]
    fn nav_ids_round_trip_through_the_registry() {
        let spec = default_menu(&registry_with(&[
            ("reports", "Reports"),
            ("admin", "Admin"),
        ]));

        assert_eq!(
            action_for_id(&spec, &nav_id("reports")),
            Some(MenuAction::NavigateTo("reports".to_string()))
        );
        assert_eq!(
            action_for_id(&spec, &nav_id("admin")),
            Some(MenuAction::NavigateTo("admin".to_string()))
        );

        // Labels come from AppDescriptor::title, not the id.
        let labels: Vec<&str> = spec
            .submenus
            .iter()
            .find(|(label, _)| label == "View")
            .expect("View submenu")
            .1
            .iter()
            .filter_map(|entry| match entry {
                MenuEntry::Item { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .collect();
        // Registration order, so this is an equality assertion, not a containment one.
        assert_eq!(labels, vec!["Reports", "Admin"]);
    }

    #[test]
    fn ids_are_unique_across_the_whole_spec() {
        // A duplicate id makes one of the two menu items silently dead: `action_for_id`
        // returns the first match and the second item can never fire its own action.
        let spec = default_menu(&registry_with(&[
            ("reports", "Reports"),
            ("admin", "Admin"),
            ("audit", "Audit"),
        ]));

        let ids: Vec<&str> = spec.ids().collect();
        let unique: HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len(), "duplicate menu ids in {ids:?}");
    }

    #[test]
    fn app_menu_actions_are_bound() {
        let spec = default_menu(&AppRegistry::new());

        assert_eq!(action_for_id(&spec, ID_RELOAD), Some(MenuAction::Reload));
        assert_eq!(
            action_for_id(&spec, ID_TOGGLE_DEVTOOLS),
            Some(MenuAction::ToggleDevTools)
        );
    }

    #[test]
    fn empty_registry_yields_no_view_submenu() {
        let spec = default_menu(&AppRegistry::new());

        assert!(
            !spec.submenus.iter().any(|(label, _)| label == "View"),
            "an empty registry must not produce an empty View submenu"
        );
        // The App submenu is still there, so the menu bar is never empty.
        assert_eq!(spec.submenus.len(), 1);
        assert_eq!(spec.submenus[0].0, "App");
    }

    #[test]
    fn quit_and_separator_carry_no_id() {
        assert_eq!(MenuEntry::Quit.id(), None);
        assert_eq!(MenuEntry::Separator.id(), None);

        let spec = default_menu(&AppRegistry::new());
        // ID_QUIT is reserved for a caller building a custom spec; the default menu uses
        // the platform's predefined quit item, which reports no id of ours.
        assert_eq!(action_for_id(&spec, ID_QUIT), None);
    }

    #[test]
    fn a_custom_spec_can_bind_quit_by_id() {
        let spec = MenuSpec::new().submenu(
            "File",
            vec![MenuEntry::item(ID_QUIT, "Exit", MenuAction::Quit)],
        );

        assert_eq!(action_for_id(&spec, ID_QUIT), Some(MenuAction::Quit));
    }
}
