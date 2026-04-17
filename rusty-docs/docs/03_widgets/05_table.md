## Table

A data table with columns, rows, and optional sorting.

### Constructor

```rust
Table::new(vec![
    Column { key: "name".into(), label: "Name".into() },
    Column { key: "age".into(), label: "Age".into() },
])
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Columns | `new(cols)` | `Vec<Column>` | Column definitions |
| Rows | `.rows(r)` | `Vec<Row>` | Data rows |
| Sort By | `.sort_by(key)` | `&str` | Default sort column |

### Example

```rust
use rusty::widgets::table::{Column, Row};
use std::collections::HashMap;

Table::new(vec![
    Column { key: "name".into(), label: "Name".into() },
    Column { key: "role".into(), label: "Role".into() },
])
.rows(vec![
    Row { values: HashMap::from([("name".into(), "Alice".into()), ("role".into(), "Admin".into())]) },
    Row { values: HashMap::from([("name".into(), "Bob".into()), ("role".into(), "User".into())]) },
])
.into()
```
