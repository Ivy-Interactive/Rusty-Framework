use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub key: String,
    pub label: String,
    pub sortable: bool,
}

/// A data table widget with sorting and filtering support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub columns: Vec<Column>,
    pub rows: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<String>,
    pub sort_ascending: bool,
}

impl Table {
    pub fn new(columns: Vec<Column>) -> Self {
        Table {
            columns,
            rows: Vec::new(),
            sort_by: None,
            sort_ascending: true,
        }
    }

    pub fn rows(mut self, rows: Vec<serde_json::Value>) -> Self {
        self.rows = rows;
        self
    }

    pub fn sort_by(mut self, column: &str, ascending: bool) -> Self {
        self.sort_by = Some(column.to_string());
        self.sort_ascending = ascending;
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for Table {
    fn widget_type(&self) -> &str {
        "table"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "table",
            "columns": self.columns,
            "rows": self.rows,
            "sortBy": self.sort_by,
            "sortAscending": self.sort_ascending,
        })
    }

    fn clone_box(&self) -> Box<dyn WidgetData> {
        Box::new(self.clone())
    }
}

impl From<Table> for Element {
    fn from(table: Table) -> Self {
        table.into_element()
    }
}
