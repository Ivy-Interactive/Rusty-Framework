use rusty::prelude::*;
use rusty::widgets::table::Column;

use crate::apps::AppEntry;
use crate::sample_base::{demo_section, sample_page};

pub fn entry() -> AppEntry {
    AppEntry {
        id: "table",
        title: "Table",
        icon: "table",
        group: "Widgets",
        order: 2,
        factory: build,
    }
}

fn build(ctx: &mut BuildContext) -> Element {
    let sort_col = use_state(ctx, "name".to_string());
    let sort_asc = use_state(ctx, true);

    let col_val = sort_col.get();
    let asc_val = sort_asc.get();

    sample_page(
        "Table",
        "Demonstrates the Table widget with columns, rows, and sorting.",
        Layout::vertical()
            .gap(16.0)
            .child(demo_section(
                "Basic Table",
                Table::new(vec![
                    Column {
                        key: "name".into(),
                        label: "Name".into(),
                        sortable: true,
                    },
                    Column {
                        key: "role".into(),
                        label: "Role".into(),
                        sortable: true,
                    },
                    Column {
                        key: "status".into(),
                        label: "Status".into(),
                        sortable: false,
                    },
                ])
                .rows(vec![
                    serde_json::json!({"name": "Alice", "role": "Engineer", "status": "Active"}),
                    serde_json::json!({"name": "Bob", "role": "Designer", "status": "Away"}),
                    serde_json::json!({"name": "Charlie", "role": "Manager", "status": "Active"}),
                    serde_json::json!({"name": "Diana", "role": "Engineer", "status": "Active"}),
                ])
                .sort_by(&col_val, asc_val)
                .into(),
            ))
            .child(demo_section(
                "Sort Controls",
                Layout::horizontal()
                    .gap(8.0)
                    .child(TextBlock::new(&format!(
                        "Sorting by: {} ({})",
                        col_val,
                        if asc_val { "ascending" } else { "descending" }
                    )))
                    .child({
                        let sc = sort_col.clone();
                        Button::new("Sort by Name").on_click(move || {
                            sc.set("name".to_string());
                        })
                    })
                    .child({
                        let sc = sort_col.clone();
                        Button::new("Sort by Role").on_click(move || {
                            sc.set("role".to_string());
                        })
                    })
                    .child({
                        let sa = sort_asc.clone();
                        Button::new("Toggle Direction").on_click(move || {
                            sa.update(|v| !v);
                        })
                    })
                    .into(),
            ))
            .into(),
    )
}
