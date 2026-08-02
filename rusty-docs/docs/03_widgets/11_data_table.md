## DataTable

A typed data grid with per-column formatting, sorting, filtering and cell events.
Where [Table](05_table.md) takes plain string cells, `DataTable` declares a type
per column so the frontend can format numbers, booleans, dates, icons and links.

### Constructor

```rust
DataTable::new(vec![
    DataTableColumn::new("name", "Name", ColType::Text),
    DataTableColumn::new("age", "Age", ColType::Number),
])
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Columns | `new(cols)` | `Vec<DataTableColumn>` | Column definitions |
| Rows | `.rows(r)` | `Vec<serde_json::Value>` | Row objects keyed by column name |
| Config | `.config(c)` | `DataTableConfig` | Table-wide behaviour flags |
| Width | `.width(w)` | `Size` | Table width |
| Height | `.height(h)` | `Size` | Table height |
| On Cell Click | `.on_cell_click(f)` | `Fn(CellClickArgs)` | Fires when a cell is clicked |
| On Row Action | `.on_row_action(f)` | `Fn(RowActionArgs)` | Fires when a row action is triggered |

### Columns

`DataTableColumn::new(name, header, col_type)` starts sortable and filterable
with `order` 0. `name` must match the key used in each row object.

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Type | `new(.., col_type)` | `ColType` | `Number`, `Text`, `Boolean`, `Date`, `DateTime`, `Icon`, `Labels` or `Link` |
| Width | `.width(w)` | `Size` | Column width |
| Hidden | `.hidden(b)` | `bool` | Omit the column from rendering |
| Sortable | `.sortable(b)` | `bool` | Allow sorting on this column (default `true`) |
| Sort Direction | `.sort_direction(d)` | `SortDirection` | `Ascending`, `Descending` or `None` |
| Filterable | `.filterable(b)` | `bool` | Allow filtering on this column (default `true`) |
| Align | `.align(a)` | `Align` | Cell alignment |
| Wrap Text | `.wrap_text(b)` | `bool` | Wrap long values instead of clipping |
| Order | `.order(n)` | `i32` | Display order |
| Icon | `.icon(i)` | `Icon` | Icon shown in the header |
| Help | `.help(s)` | `&str` | Header tooltip |
| Color | `.color(c)` | `Color` | Header color |

### Config

`DataTableConfig::new()` enables sorting, filtering, column reordering, column
resizing, copy-selection and vertical borders, with `SelectionMode::Cells`.

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Freeze Columns | `.freeze_columns(n)` | `usize` | Pin the first `n` columns |
| Allow Sorting | `.allow_sorting(b)` | `bool` | Table-wide sorting toggle |
| Allow Filtering | `.allow_filtering(b)` | `bool` | Table-wide filtering toggle |
| Allow Column Reordering | `.allow_column_reordering(b)` | `bool` | Let users drag columns |
| Allow Column Resizing | `.allow_column_resizing(b)` | `bool` | Let users resize columns |
| Allow Copy Selection | `.allow_copy_selection(b)` | `bool` | Let users copy the selection |
| Selection Mode | `.selection_mode(m)` | `SelectionMode` | `None`, `Rows`, `Columns` or `Cells` |
| Show Index Column | `.show_index_column(b)` | `bool` | Render a row-number column |
| Show Groups | `.show_groups(b)` | `bool` | Render column groups |
| Show Column Type Icons | `.show_column_type_icons(b)` | `bool` | Show a type icon per header |
| Show Vertical Borders | `.show_vertical_borders(b)` | `bool` | Draw borders between columns |
| Show Search | `.show_search(b)` | `bool` | Render a search box |
| Id Column Name | `.id_column_name(s)` | `&str` | Column supplying each row's identity |

### Events

`on_cell_click` receives `CellClickArgs { row_index, column_index, column_name,
cell_value, row_id }`. `row_id` is populated from the column named by
`id_column_name`.

`on_row_action` receives `RowActionArgs { id, tag }`, where `tag` identifies
which action was triggered.

### Example

```rust
use serde_json::json;

DataTable::new(vec![
    DataTableColumn::new("name", "Name", ColType::Text),
    DataTableColumn::new("age", "Age", ColType::Number).align(Align::End),
    DataTableColumn::new("active", "Active", ColType::Boolean),
])
.rows(vec![
    json!({"id": 1, "name": "Alice", "age": 30, "active": true}),
    json!({"id": 2, "name": "Bob", "age": 25, "active": false}),
])
.config(
    DataTableConfig::new()
        .show_search(true)
        .id_column_name("id")
        .selection_mode(SelectionMode::Rows),
)
.on_cell_click(|args| {
    println!("clicked {} on row {}", args.column_name, args.row_index);
})
.into()
```

### Server-side filtering

`apply_filter` runs a query in the same grammar Ivy's filter editor uses and
returns a table holding only the matching rows. See
[Filters](../02_concepts/07_filters.md) for the grammar itself.

```rust
let table = DataTable::new(vec![
    DataTableColumn::new("name", "Name", ColType::Text),
    DataTableColumn::new("age", "Age", ColType::Number),
])
.rows(vec![
    json!({"name": "Alice", "age": 30}),
    json!({"name": "Bob", "age": 25}),
]);

let filtered = table.apply_filter("[age] > 28").expect("valid query");
assert_eq!(filtered.rows.len(), 1);
```

| Method | Returns | Description |
|--------|---------|-------------|
| `.filter_columns()` | `Vec<ColumnDef>` | The columns a query may name, as the grammar sees them |
| `.apply_filter(q)` | `Result<DataTable, Vec<ParseError>>` | Keep the rows matching `q`; `Err` carries the parse and validation errors |

A column is offered to `filter_columns` only when it is `filterable` and not
`hidden`, which is the same test the frontend applies before handing columns to
its editor. Naming an excluded column therefore gets
`Column 'x' does not exist` on both sides rather than working on one of them.
Each `ColType` maps onto one of the grammar's five types: `Number` to `number`,
`Boolean` to `boolean`, `Date` and `DateTime` to `date`, and `Text`, `Icon`,
`Labels` and `Link` to `string`.

An empty or whitespace-only query is valid and keeps every row, so clearing a
filter needs no special case.

### Limitations

Rows travel inline in the widget JSON, exactly as `Table` does: the whole set is
serialized on every build. Ivy's `DataTableConnection` — the server-side query
pipeline that pages, sorts and filters large datasets on demand — is not ported,
so paginate in your own code before handing rows to the widget.

Filtering is available, but only when **your code** calls `apply_filter`. The
filter box in the rendered table sends its query to a gRPC `DataTableService`
that Rusty does not implement, so typing there still goes nowhere; the same
applies to sorting and to the search box. Treat `allow_filtering`,
`allow_sorting` and `show_search` as frontend chrome until that service exists.
