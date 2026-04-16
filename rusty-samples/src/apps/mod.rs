pub mod concepts;
pub mod demos;
pub mod widgets;

use rusty::prelude::*;

/// Metadata for a registered sample app.
pub struct AppEntry {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: &'static str,
    pub group: &'static str,
    pub order: u32,
    pub factory: fn(&mut BuildContext) -> Element,
}

/// Collect all registered sample apps, sorted by group then order.
pub fn all_apps() -> Vec<AppEntry> {
    let mut apps = Vec::new();
    apps.extend(demos::register());
    apps.extend(concepts::register());
    apps.extend(widgets::register());
    apps.sort_by(|a, b| a.group.cmp(b.group).then(a.order.cmp(&b.order)));
    apps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_apps_registered() {
        let apps = all_apps();
        assert!(
            apps.len() >= 15,
            "Expected at least 15 apps, got {}",
            apps.len()
        );
    }

    #[test]
    fn test_apps_sorted_by_group_and_order() {
        let apps = all_apps();
        for window in apps.windows(2) {
            let a = &window[0];
            let b = &window[1];
            assert!(
                (a.group, a.order) <= (b.group, b.order),
                "Apps not sorted: ({}, {}) > ({}, {})",
                a.group,
                a.order,
                b.group,
                b.order
            );
        }
    }

    #[test]
    fn test_unique_app_ids() {
        let apps = all_apps();
        let mut ids: Vec<&str> = apps.iter().map(|a| a.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), apps.len(), "Duplicate app IDs found");
    }
}
