## Filters

`rusty_filter` parses the filter query language Ivy's filter editor uses, into an
AST you can validate against a column schema, evaluate over rows, and print back
to a canonical string. It is what makes a filter typed in one place mean the same
thing in the other.

```rust
use rusty::prelude::*;

let columns = vec![
    ColumnDef::new("age", ColumnType::Number),
    ColumnDef::new("name", ColumnType::String),
];
let result = parse_query("[age] > 30 AND [name] starts with \"A\"", &columns);
let filter = result.filters.expect("the query is valid");

let rows = vec![
    serde_json::json!({"age": 41, "name": "Ada"}),
    serde_json::json!({"age": 41, "name": "Bob"}),
    serde_json::json!({"age": 12, "name": "Ann"}),
];
assert_eq!(retain_matching(&filter, rows, &columns).len(), 1);
```

`parse_query` returns a `ParseResult` carrying **either** `filters` or `errors`,
never both. Empty and whitespace-only input is valid and matches every row, so a
cleared filter box needs no special case.

### Column references

A column is named in square brackets: `[age]`. Whatever is between the brackets
is the name, verbatim — spaces and unicode included, so `[first name]` works and
`[ age ]` is a different column from `[age]`. Names are matched case-sensitively;
keywords are not. `[]` is a syntax error, but `[ ]` is the column named `" "`.

### Operators

Every operator has a symbolic and a spelled-out form, and the spelled-out forms
are case-insensitive. Both parse to one `FilterFunction`:

| Function | Spellings |
|----------|-----------|
| `Equals` | `=`, `==`, `equals` |
| `Equals` negated | `!=`, `not equals`, `not equal` |
| `GreaterThan` | `>`, `greater than` |
| `GreaterThanOrEqual` | `>=`, `greater than or equal` |
| `LessThan` | `<`, `less than` |
| `LessThanOrEqual` | `<=`, `less than or equal` |
| `Contains` | `contains`, `not contains` |
| `StartsWith` | `starts with`, `not starts with` |
| `EndsWith` | `ends with`, `not ends with` |
| `IsBlank` | `is blank` |
| `IsNotBlank` | `is not blank` |

There is no `NotEquals` function: `!=` is `Equals` with `negate: true`. The same
goes for the negated text operators.

Two spellings that look like they should work and do not: bare `equal` is a
syntax error where `equals` is fine, though `not equal` is accepted; and the
`or equal` tail of `>=` and `<=` is singular, so `greater than or equals` is
rejected.

### Which operators a column type allows

`ColumnType` has five variants, and validation rejects a mismatch with
`Operator 'x' is not compatible with type 'y'`:

| Column type | Allowed |
|-------------|---------|
| `String` | `equals`, `contains`, `starts with`, `ends with`, `is blank`, `is not blank` |
| `Number` | `equals`, `>`, `>=`, `<`, `<=` |
| `Date` | `equals`, `>`, `>=`, `<`, `<=`, `is blank`, `is not blank` |
| `Boolean` | `equals` |
| `Enum` | `equals`, `is blank`, `is not blank` |

`ColumnType::normalize` maps a backend type name onto one of the five:
`INT32`, `INT64`, `DOUBLE`, `DECIMAL` and `NUMBER` become `Number`; `TEXT`,
`STRING` and `ICON` become `String`; `DATE` and `DATETIME` become `Date`; and any
name it does not recognize becomes `String`.

### Literals

| Kind | Examples | Notes |
|------|----------|-------|
| Number | `42`, `-5`, `1.5`, `007` | One JavaScript number type, so `007` is `7` and `1.000` is `1`. `1e5` is a lexer error. |
| String | `"Ada"`, `"a\"b"`, `"a\\b"` | Double quotes only. `\"`, `\'` and `\\` are the only escapes; anything else keeps its backslash, so `\t` stays two characters rather than becoming a tab. A raw newline inside a string is an error, but a raw tab is allowed. |
| Boolean | `true`, `false` | Case-insensitive. |
| Date | `"2024-01-01"`, `"2024-01-01T10:20:30.123Z"` | A quoted string on a `Date` column. Validation is shape-only, so `"2024-13-99"` passes it and then matches nothing. `"2024-1-1"` is rejected — pad to two digits. |

The value type has to match the column, or validation reports
`Expected number for column 'age', got string`.

### Combining and precedence

`AND` binds tighter than `OR`, and both are case-insensitive. Parentheses
override precedence and are **never collapsed**: `(([age] > 1))` keeps both
group levels in the AST, because the frontend renders those groups.

`NOT` negates the filter that follows and toggles rather than accumulates, so
`not not [age] > 1` is the same filter as `[age] > 1`.

```rust
"[age] > 1 AND [name] = \"a\" OR [active] = true"
// parses as: ([age] > 1 AND [name] = "a") OR [active] = true
```

The AST reflects that: the root is an `OR` group whose first arm is a nested
`AND` group. Same-operator chains stay flat — a three-arm `AND` is one group
with three filters, not two nested ones.

### Evaluating

| Function | Use |
|----------|-----|
| `evaluate(&filter, &row, &columns)` | Does one row match? |
| `retain_matching(&filter, rows, &columns)` | Keep the matching rows, in order. |
| `count_matches(&filter, &rows, &columns)` | Count without allocating. |

Evaluation is strict about types: `equals` on a `String` column does not match
the number `1` against `"1"`. `contains`, `starts with` and `ends with` are
case-sensitive. A `null` or absent column value fails every operator except
`is blank`. An empty `AND` group matches everything — and so does an empty `OR`
group, which is the reference's behaviour rather than the `false` you might
expect.

### Canonical strings and cache keys

`to_query_string` prints an AST back to a query, and `canonical_key` is that
same function under the name that says what it is for: two equivalent filters
produce one string, so they can share one cache entry.

```rust
use rusty::core::QueryService;

let filter = parse_query_unchecked("[age] greater than 30").unwrap();
assert_eq!(QueryService::filtered_key("people", Some(&filter)), "people?[age] > 30");
```

The canonical spelling is not simply the input echoed back. It is symbolic for
the orderings and spelled out for equality — `[age] greater than 30` prints as
`[age] > 30` while `[age] = 1` prints as `[age] equals 1` — because that is what
the frontend's `formatQuery` does, and the two sides have to agree.

Printing is idempotent from the second pass onwards, but the first pass can
reshape the AST: a negated leaf prints as `NOT (...)`, and re-parsing reads those
parentheses as a group, moving the negation onto a wrapping group filter. The
filter still means the same thing, and the reference does the same, so compare
canonical strings rather than ASTs when you need to know whether two filters
match.

### Compatibility with the frontend editor

The AST serializes to exactly the JSON `filter-query-editor` produces —
`{"filters":{"op":"AND","filters":[{"condition":{"column":"age","function":"greaterThan","args":[1]}}]}}`
— key presence included, which is finer than it sounds. A comparison omits the
`negate` key altogether; a text operation emits `negate: false`. Behaviour was
matched against version 2.2.0 over roughly a hundred inputs, and
`rusty-filter/tests/frontend_ast_compat.rs` pins the shape against captured
bundle output.

**The browser's filter box does not reach this crate.** The rendered `DataTable`
sends filter queries to a gRPC `DataTableService` that Rusty does not implement,
so typing in the filter box goes nowhere. What works today is Rust-side
filtering you call yourself, via [`DataTable::apply_filter`](../03_widgets/11_data_table.md)
or the functions above. This crate is what a future `DataTableService` would
parse with.

### Not implemented

- The gRPC `DataTableService` (`Query`, `ParseFilter`, `Distinct`).
- `parseInvalidQuery` and its LLM-based query repair.
- Sorting, aggregations, pagination and column selection — the rest of Ivy's
  `DataTableQuery`.
- Multiple errors for one input. ANTLR recovers from a syntax error and can
  report a cascade; this parser reports the first and stops. Semantic errors are
  reported in full, one per offending condition.
- Error spans are byte offsets, not UTF-16 code units. They agree for ASCII and
  part company once a multi-byte character appears before the error.
