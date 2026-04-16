use rusty::prelude::*;
use rusty::widgets::button::ButtonVariant;
use rusty::widgets::text::TextVariant;

use crate::apps::{all_apps, AppEntry};

/// The root app shell with sidebar navigation and content area.
pub struct AppShell;

impl View for AppShell {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let apps = all_apps();
        let active_id = use_state(ctx, apps[0].id.to_string());

        // Group apps by category
        let mut groups: Vec<(&str, Vec<&AppEntry>)> = Vec::new();
        for app in &apps {
            if let Some(group) = groups.last_mut().filter(|(g, _)| *g == app.group) {
                group.1.push(app);
            } else {
                groups.push((app.group, vec![app]));
            }
        }

        // Build sidebar
        let mut sidebar = Layout::vertical().gap(4.0).padding(16.0);

        sidebar = sidebar.child(
            TextBlock::new("Rusty Samples")
                .variant(TextVariant::Heading2)
                .bold(),
        );

        for (group_name, group_apps) in &groups {
            sidebar = sidebar.child(
                Layout::vertical().gap(2.0).child(
                    TextBlock::new(group_name)
                        .variant(TextVariant::Caption)
                        .bold(),
                ),
            );

            for app in group_apps {
                let app_id = app.id;
                let is_active = active_id.get() == app_id;
                let nav_state = active_id.clone();

                let variant = if is_active {
                    ButtonVariant::Secondary
                } else {
                    ButtonVariant::Ghost
                };

                sidebar = sidebar.child(
                    Button::new(app.title)
                        .variant(variant)
                        .icon(app.icon)
                        .on_click(move || {
                            nav_state.set(app_id.to_string());
                        }),
                );
            }
        }

        // Build content area - find the active app and render it
        let current_id = active_id.get();
        let content = apps
            .iter()
            .find(|a| a.id == current_id)
            .map(|app| (app.factory)(ctx))
            .unwrap_or_else(|| TextBlock::new("Select an app").into());

        Layout::horizontal()
            .child(sidebar)
            .child(Layout::vertical().gap(0.0).child(content))
            .into()
    }
}
