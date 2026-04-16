use rusty::prelude::*;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "context",
        title: "Context",
        icon: "share-2",
        group: "Concepts",
        order: 4,
        factory: build,
    }
}

#[derive(Clone, Debug)]
struct ThemeContext {
    name: String,
    primary_color: String,
}

fn build(ctx: &mut BuildContext) -> Element {
    let theme_index = use_state(ctx, 0usize);

    let themes = [
        ThemeContext {
            name: "Blue".to_string(),
            primary_color: "#3b82f6".to_string(),
        },
        ThemeContext {
            name: "Green".to_string(),
            primary_color: "#22c55e".to_string(),
        },
        ThemeContext {
            name: "Purple".to_string(),
            primary_color: "#a855f7".to_string(),
        },
    ];

    let idx = theme_index.get();
    let current_theme = themes[idx % themes.len()].clone();

    // Provide context for descendant views
    create_context(ctx, current_theme.clone());

    let switch = theme_index.clone();

    sample_page(
        "Context",
        "Demonstrates create_context and use_context for sharing state across views.",
        Layout::vertical()
            .gap(16.0)
            .child(demo_section(
                "Theme Provider",
                Layout::vertical()
                    .gap(8.0)
                    .child(TextBlock::new(&format!(
                        "Current theme: {} ({})",
                        current_theme.name, current_theme.primary_color
                    )))
                    .child(Button::new("Switch Theme").on_click(move || {
                        switch.update(|v| v + 1);
                    }))
                    .child(TextBlock::paragraph(
                        "The theme is provided via create_context and consumed by child components with use_context.",
                    ))
                    .into(),
            ))
            .child(demo_section(
                "Theme Consumer",
                Layout::vertical()
                    .gap(8.0)
                    .child(
                        Badge::new(&format!("Theme: {}", current_theme.name))
                            .color(Color::hex(&current_theme.primary_color)),
                    )
                    .child(TextBlock::paragraph(
                        "This badge reads the theme from context to determine its color.",
                    ))
                    .into(),
            ))
            .into(),
    )
}
