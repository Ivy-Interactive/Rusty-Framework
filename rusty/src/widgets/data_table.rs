use crate::core::event_registry::EventRegistry;
use crate::shared::{Align, Color, Icon, Size};
use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// The value type of a `DataTableColumn`, driving how the frontend formats cells.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColType {
    Number,
    #[default]
    Text,
    Boolean,
    Date,
    DateTime,
    Icon,
    Labels,
    Link,
}

/// Sort direction applied to a column.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortDirection {
    Ascending,
    Descending,
    #[default]
    None,
}

/// Which parts of the table the user may select.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionMode {
    None,
    Rows,
    Columns,
    #[default]
    Cells,
}

/// A single column definition of a `DataTable`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataTableColumn {
    pub name: String,
    pub header: String,
    #[serde(rename = "type")]
    pub col_type: ColType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Size>,
    pub hidden: bool,
    pub sortable: bool,
    pub sort_direction: SortDirection,
    pub filterable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    pub wrap_text: bool,
    pub order: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}

impl DataTableColumn {
    pub fn new(name: &str, header: &str, col_type: ColType) -> Self {
        DataTableColumn {
            name: name.to_string(),
            header: header.to_string(),
            col_type,
            width: None,
            hidden: false,
            sortable: true,
            sort_direction: SortDirection::None,
            filterable: true,
            align: None,
            wrap_text: false,
            order: 0,
            icon: None,
            help: None,
            color: None,
        }
    }

    pub fn width(mut self, width: Size) -> Self {
        self.width = Some(width);
        self
    }

    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    pub fn sort_direction(mut self, direction: SortDirection) -> Self {
        self.sort_direction = direction;
        self
    }

    pub fn filterable(mut self, filterable: bool) -> Self {
        self.filterable = filterable;
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = Some(align);
        self
    }

    pub fn wrap_text(mut self, wrap_text: bool) -> Self {
        self.wrap_text = wrap_text;
        self
    }

    pub fn order(mut self, order: i32) -> Self {
        self.order = order;
        self
    }

    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn help(mut self, help: &str) -> Self {
        self.help = Some(help.to_string());
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// Table-wide behaviour flags.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataTableConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freeze_columns: Option<usize>,
    pub allow_sorting: bool,
    pub allow_filtering: bool,
    pub allow_column_reordering: bool,
    pub allow_column_resizing: bool,
    pub allow_copy_selection: bool,
    pub selection_mode: SelectionMode,
    pub show_index_column: bool,
    pub show_groups: bool,
    pub show_column_type_icons: bool,
    pub show_vertical_borders: bool,
    pub show_search: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_column_name: Option<String>,
}

impl Default for DataTableConfig {
    fn default() -> Self {
        DataTableConfig {
            freeze_columns: None,
            allow_sorting: true,
            allow_filtering: true,
            allow_column_reordering: true,
            allow_column_resizing: true,
            allow_copy_selection: true,
            selection_mode: SelectionMode::Cells,
            show_index_column: false,
            show_groups: false,
            show_column_type_icons: false,
            show_vertical_borders: true,
            show_search: false,
            id_column_name: None,
        }
    }
}

impl DataTableConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn freeze_columns(mut self, count: usize) -> Self {
        self.freeze_columns = Some(count);
        self
    }

    pub fn allow_sorting(mut self, allow: bool) -> Self {
        self.allow_sorting = allow;
        self
    }

    pub fn allow_filtering(mut self, allow: bool) -> Self {
        self.allow_filtering = allow;
        self
    }

    pub fn allow_column_reordering(mut self, allow: bool) -> Self {
        self.allow_column_reordering = allow;
        self
    }

    pub fn allow_column_resizing(mut self, allow: bool) -> Self {
        self.allow_column_resizing = allow;
        self
    }

    pub fn allow_copy_selection(mut self, allow: bool) -> Self {
        self.allow_copy_selection = allow;
        self
    }

    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    pub fn show_index_column(mut self, show: bool) -> Self {
        self.show_index_column = show;
        self
    }

    pub fn show_groups(mut self, show: bool) -> Self {
        self.show_groups = show;
        self
    }

    pub fn show_column_type_icons(mut self, show: bool) -> Self {
        self.show_column_type_icons = show;
        self
    }

    pub fn show_vertical_borders(mut self, show: bool) -> Self {
        self.show_vertical_borders = show;
        self
    }

    pub fn show_search(mut self, show: bool) -> Self {
        self.show_search = show;
        self
    }

    pub fn id_column_name(mut self, name: &str) -> Self {
        self.id_column_name = Some(name.to_string());
        self
    }
}

/// Arguments delivered to `DataTable::on_cell_click`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CellClickArgs {
    pub row_index: usize,
    pub column_index: usize,
    pub column_name: String,
    pub cell_value: serde_json::Value,
    pub row_id: Option<serde_json::Value>,
}

/// Arguments delivered to `DataTable::on_row_action`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RowActionArgs {
    pub id: Option<serde_json::Value>,
    pub tag: Option<String>,
}

/// A typed data grid with per-column formatting, sorting and cell events.
///
/// Rows travel inline in the widget JSON, exactly as [`crate::widgets::Table`] does.
/// Ivy's `DataTableConnection` server-side query pipeline is not ported.
#[derive(Clone, Serialize, Deserialize)]
pub struct DataTable {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub columns: Vec<DataTableColumn>,
    pub rows: Vec<serde_json::Value>,
    pub config: DataTableConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Size>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<Size>,
    #[serde(skip)]
    pub on_cell_click: Option<Arc<dyn Fn(CellClickArgs) + Send + Sync>>,
    #[serde(skip)]
    pub on_row_action: Option<Arc<dyn Fn(RowActionArgs) + Send + Sync>>,
}

impl std::fmt::Debug for DataTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataTable")
            .field("columns", &self.columns)
            .field("rows", &self.rows.len())
            .finish()
    }
}

impl DataTable {
    pub fn new(columns: Vec<DataTableColumn>) -> Self {
        DataTable {
            id: None,
            columns,
            rows: Vec::new(),
            config: DataTableConfig::default(),
            width: None,
            height: None,
            on_cell_click: None,
            on_row_action: None,
        }
    }

    pub fn rows(mut self, rows: Vec<serde_json::Value>) -> Self {
        self.rows = rows;
        self
    }

    pub fn config(mut self, config: DataTableConfig) -> Self {
        self.config = config;
        self
    }

    pub fn width(mut self, width: Size) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: Size) -> Self {
        self.height = Some(height);
        self
    }

    pub fn on_cell_click(
        mut self,
        handler: impl Fn(CellClickArgs) + Send + Sync + 'static,
    ) -> Self {
        self.on_cell_click = Some(Arc::new(handler));
        self
    }

    pub fn on_row_action(
        mut self,
        handler: impl Fn(RowActionArgs) + Send + Sync + 'static,
    ) -> Self {
        self.on_row_action = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for DataTable {
    fn widget_type(&self) -> &str {
        "data_table"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "data_table",
            "id": self.id,
            "columns": self.columns,
            "rows": self.rows,
            "config": self.config,
            "width": self.width,
            "height": self.height,
            "hasOnCellClick": self.on_cell_click.is_some(),
            "hasOnRowAction": self.on_row_action.is_some(),
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
        if let Some(handler) = &self.on_cell_click {
            let handler = handler.clone();
            registry.register(
                widget_id,
                "cellclick",
                Arc::new(move |args| {
                    if let Ok(parsed) = serde_json::from_value::<CellClickArgs>(args) {
                        handler(parsed);
                    }
                }),
            );
        }
        if let Some(handler) = &self.on_row_action {
            let handler = handler.clone();
            registry.register(
                widget_id,
                "rowaction",
                Arc::new(move |args| {
                    if let Ok(parsed) = serde_json::from_value::<RowActionArgs>(args) {
                        handler(parsed);
                    }
                }),
            );
        }
    }
}

impl From<DataTable> for Element {
    fn from(table: DataTable) -> Self {
        table.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::BuildContext;
    use std::sync::Mutex;

    fn sample_columns() -> Vec<DataTableColumn> {
        vec![
            DataTableColumn::new("name", "Name", ColType::Text),
            DataTableColumn::new("age", "Age", ColType::Number),
        ]
    }

    #[test]
    fn test_data_table_builder_round_trip() {
        let table = DataTable::new(sample_columns())
            .rows(vec![json!({"name": "Alice", "age": 30})])
            .width(Size::Percent(100.0))
            .height(Size::Px(400.0));

        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.width, Some(Size::Percent(100.0)));
        assert_eq!(table.height, Some(Size::Px(400.0)));
    }

    #[test]
    fn test_data_table_column_builder_round_trip() {
        let col = DataTableColumn::new("email", "Email", ColType::Link)
            .width(Size::Px(200.0))
            .hidden(true)
            .sortable(false)
            .sort_direction(SortDirection::Descending)
            .filterable(false)
            .align(Align::End)
            .wrap_text(true)
            .order(3)
            .icon("mail")
            .help("Contact address")
            .color(Color::hex("#ff0000"));

        assert_eq!(col.name, "email");
        assert_eq!(col.header, "Email");
        assert_eq!(col.col_type, ColType::Link);
        assert_eq!(col.width, Some(Size::Px(200.0)));
        assert!(col.hidden);
        assert!(!col.sortable);
        assert_eq!(col.sort_direction, SortDirection::Descending);
        assert!(!col.filterable);
        assert_eq!(col.align, Some(Align::End));
        assert!(col.wrap_text);
        assert_eq!(col.order, 3);
        assert_eq!(col.icon, Some(Icon::new("mail")));
        assert_eq!(col.help.as_deref(), Some("Contact address"));
        assert_eq!(col.color, Some(Color::hex("#ff0000")));
    }

    #[test]
    fn test_data_table_to_json_keys() {
        let table = DataTable::new(sample_columns())
            .rows(vec![json!({"name": "Alice", "age": 30})])
            .on_cell_click(|_| {})
            .on_row_action(|_| {});

        let json = table.to_json();
        assert_eq!(json["type"], "data_table");
        assert_eq!(json["hasOnCellClick"], true);
        assert_eq!(json["hasOnRowAction"], true);
        assert_eq!(json["rows"][0]["name"], "Alice");
        assert_eq!(json["config"]["allowSorting"], true);
        assert_eq!(json["config"]["selectionMode"], "cells");
    }

    #[test]
    fn test_data_table_json_without_handlers() {
        let json = DataTable::new(sample_columns()).to_json();
        assert_eq!(json["hasOnCellClick"], false);
        assert_eq!(json["hasOnRowAction"], false);
    }

    #[test]
    fn test_column_col_type_serializes_under_type_key() {
        let json =
            serde_json::to_value(DataTableColumn::new("age", "Age", ColType::Number)).unwrap();
        assert_eq!(json["type"], "number");
        assert_eq!(json["name"], "age");
        assert_eq!(json["header"], "Age");
        assert_eq!(json["sortDirection"], "none");
        assert_eq!(json["wrapText"], false);
        assert!(json.get("colType").is_none());
    }

    #[test]
    fn test_data_table_assign_ids() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let mut element: Element = DataTable::new(sample_columns()).into();
        element.assign_ids(&mut ctx);
        if let Element::Widget(ref w) = element {
            assert_eq!(w.get_id(), Some("w-0"));
            assert_eq!(w.to_json()["id"], "w-0");
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_data_table_cell_click_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received: Arc<Mutex<Option<CellClickArgs>>> = Arc::new(Mutex::new(None));
        let received_clone = received.clone();

        let mut element: Element = DataTable::new(sample_columns())
            .on_cell_click(move |args| {
                *received_clone.lock().unwrap() = Some(args);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        let dispatched = registry.dispatch(
            "w-0",
            "cellclick",
            json!({
                "rowIndex": 2,
                "columnIndex": 1,
                "columnName": "age",
                "cellValue": 30,
                "rowId": "row-2"
            }),
        );
        assert!(dispatched);

        let args = received
            .lock()
            .unwrap()
            .clone()
            .expect("handler not called");
        assert_eq!(args.row_index, 2);
        assert_eq!(args.column_index, 1);
        assert_eq!(args.column_name, "age");
        assert_eq!(args.cell_value, json!(30));
        assert_eq!(args.row_id, Some(json!("row-2")));
    }

    #[test]
    fn test_data_table_row_action_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received: Arc<Mutex<Option<RowActionArgs>>> = Arc::new(Mutex::new(None));
        let received_clone = received.clone();

        let mut element: Element = DataTable::new(sample_columns())
            .on_row_action(move |args| {
                *received_clone.lock().unwrap() = Some(args);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "rowaction", json!({"id": 7, "tag": "delete"})));

        let args = received
            .lock()
            .unwrap()
            .clone()
            .expect("handler not called");
        assert_eq!(args.id, Some(json!(7)));
        assert_eq!(args.tag.as_deref(), Some("delete"));
    }

    #[test]
    fn test_data_table_config_defaults() {
        let config = DataTableConfig::default();
        assert_eq!(config.freeze_columns, None);
        assert!(config.allow_sorting);
        assert!(config.allow_filtering);
        assert!(config.allow_column_reordering);
        assert!(config.allow_column_resizing);
        assert!(config.allow_copy_selection);
        assert_eq!(config.selection_mode, SelectionMode::Cells);
        assert!(!config.show_index_column);
        assert!(!config.show_groups);
        assert!(!config.show_column_type_icons);
        assert!(config.show_vertical_borders);
        assert!(!config.show_search);
        assert_eq!(config.id_column_name, None);
    }

    #[test]
    fn test_data_table_config_builder() {
        let config = DataTableConfig::new()
            .freeze_columns(2)
            .allow_sorting(false)
            .allow_filtering(false)
            .allow_column_reordering(false)
            .allow_column_resizing(false)
            .allow_copy_selection(false)
            .selection_mode(SelectionMode::Rows)
            .show_index_column(true)
            .show_groups(true)
            .show_column_type_icons(true)
            .show_vertical_borders(false)
            .show_search(true)
            .id_column_name("id");

        assert_eq!(config.freeze_columns, Some(2));
        assert!(!config.allow_sorting);
        assert_eq!(config.selection_mode, SelectionMode::Rows);
        assert!(config.show_search);
        assert_eq!(config.id_column_name.as_deref(), Some("id"));

        let table = DataTable::new(sample_columns()).config(config);
        assert_eq!(table.to_json()["config"]["freezeColumns"], 2);
    }

    #[test]
    fn test_data_table_into_element() {
        let el: Element = DataTable::new(vec![]).into();
        assert!(matches!(el, Element::Widget(_)));
    }
}
