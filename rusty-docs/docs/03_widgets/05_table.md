## Table

A data table with columns, rows, and optional sorting.

### Constructor

```rust
Table::new(vec![
    Column { key: "name".into(), label: "Name".into(), sortable: true },
    Column { key: "age".into(), label: "Age".into(), sortable: false },
])
```

Every `Column` needs all three fields; there is no `Default`.

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Columns | `new(cols)` | `Vec<Column>` | Column definitions |
| Rows | `.rows(r)` | `Vec<serde_json::Value>` | Data rows, one JSON object per row |
| Sort By | `.sort_by(column, ascending)` | `&str`, `bool` | Default sort column and direction |

Rows are plain JSON objects rather than a dedicated `Row` type: each key matches a `Column.key`, and keys with no matching column are ignored.

### Example

```rust
use rusty::widgets::table::Column;
use serde_json::json;

Table::new(vec![
    Column { key: "name".into(), label: "Name".into(), sortable: true },
    Column { key: "role".into(), label: "Role".into(), sortable: false },
])
.rows(vec![
    json!({ "name": "Alice", "role": "Admin" }),
    json!({ "name": "Bob", "role": "User" }),
])
.sort_by("name", true)
.into()
```

See it running:

```bash
cargo run --example widget_gallery
```
