//! Semantic validation against a column schema.
//!
//! A port of `SemanticValidator` and `TypeChecker`, message strings included, so
//! that a Rust-side error reads exactly like the browser's.
//!
//! Every error carries `start: 0` and `end: 0`: the reference validator has no
//! position information to report, and the frontend's own `parseQuery` passes
//! those zeros straight through. That is faithful behaviour, not a defect.

use crate::ast::{Condition, Filter, FilterFunction, FilterGroup};
use crate::column::{find_column, ColumnDef, ColumnType};
use crate::parser::ParseError;

/// Validate `group` against `columns`, returning every error found.
///
/// Validation walks nested groups. A condition naming an unknown column reports
/// only that error and is not checked further, matching the reference's early
/// return.
pub fn validate_filter_group(group: &FilterGroup, columns: &[ColumnDef]) -> Vec<ParseError> {
    let mut errors = Vec::new();
    walk_group(group, columns, &mut errors);
    errors
}

fn walk_group(group: &FilterGroup, columns: &[ColumnDef], errors: &mut Vec<ParseError>) {
    for filter in &group.filters {
        walk_filter(filter, columns, errors);
    }
}

fn walk_filter(filter: &Filter, columns: &[ColumnDef], errors: &mut Vec<ParseError>) {
    // `condition` wins when both are somehow set, as the `if / else if` does.
    if let Some(condition) = &filter.condition {
        validate_condition(condition, columns, errors);
    } else if let Some(group) = &filter.group {
        walk_group(group, columns, errors);
    }
}

fn validate_condition(condition: &Condition, columns: &[ColumnDef], errors: &mut Vec<ParseError>) {
    let Some(column) = find_column(columns, &condition.column) else {
        errors.push(error(format!(
            "Column '{}' does not exist",
            condition.column
        )));
        return;
    };
    let column_type = column.column_type;

    if condition.function.is_blank_operator() {
        if !is_blank_operator_compatible(column_type) {
            errors.push(incompatible(condition.function, column_type));
        }
        // Blank operators take no arguments, so there is nothing left to check.
        return;
    }

    if !is_operator_compatible(column_type, condition.function) {
        errors.push(incompatible(condition.function, column_type));
        return;
    }

    if condition.args.is_empty() {
        errors.push(error(format!(
            "Operator '{}' requires a value",
            condition.function.display_name()
        )));
        return;
    }

    for arg in &condition.args {
        if let Err(message) = validate_value_type(arg, column) {
            errors.push(error(message));
        }
    }
}

fn error(message: String) -> ParseError {
    ParseError::new(message, 0, 0)
}

fn incompatible(function: FilterFunction, column_type: ColumnType) -> ParseError {
    error(format!(
        "Operator '{}' is not compatible with type '{}'",
        function.display_name(),
        column_type.as_str()
    ))
}

/// Whether `function` may be applied to `column_type`.
///
/// `String` allows equals, contains, startsWith and endsWith; `Number` and
/// `Date` allow equals plus the four orderings; `Boolean` and `Enum` allow
/// equals only.
pub fn is_operator_compatible(column_type: ColumnType, function: FilterFunction) -> bool {
    use FilterFunction::*;
    match column_type {
        ColumnType::String => matches!(function, Equals | Contains | StartsWith | EndsWith),
        ColumnType::Number | ColumnType::Date => matches!(
            function,
            Equals | GreaterThan | LessThan | GreaterThanOrEqual | LessThanOrEqual
        ),
        ColumnType::Boolean | ColumnType::Enum => function == Equals,
    }
}

/// Whether the blank operators may be applied to `column_type`. They are
/// allowed on `String`, `Date` and `Enum` only.
pub fn is_blank_operator_compatible(column_type: ColumnType) -> bool {
    matches!(
        column_type,
        ColumnType::String | ColumnType::Date | ColumnType::Enum
    )
}

/// Check one argument against a column, returning the reference's error message
/// on mismatch.
pub fn validate_value_type(value: &serde_json::Value, column: &ColumnDef) -> Result<(), String> {
    let name = &column.name;
    match column.column_type {
        ColumnType::String => {
            if !value.is_string() {
                return Err(format!(
                    "Expected string for column '{name}', got {}",
                    js_typeof(value)
                ));
            }
        }
        ColumnType::Number => {
            if !value.is_number() {
                return Err(format!(
                    "Expected number for column '{name}', got {}",
                    js_typeof(value)
                ));
            }
        }
        ColumnType::Boolean => {
            if !value.is_boolean() {
                return Err(format!(
                    "Expected boolean for column '{name}', got {}",
                    js_typeof(value)
                ));
            }
        }
        ColumnType::Date => {
            let Some(text) = value.as_str() else {
                return Err(format!(
                    "Expected date string for column '{name}', got {}",
                    js_typeof(value)
                ));
            };
            if !is_iso_date_shape(text) {
                return Err(format!(
                    "Invalid date format for column '{name}'. Expected YYYY-MM-DD or ISO datetime"
                ));
            }
        }
        ColumnType::Enum => {
            if !value.is_string() {
                return Err(format!(
                    "Expected string for enum column '{name}', got {}",
                    js_typeof(value)
                ));
            }
            // The reference skips value-set validation: `ColumnDef` carries no
            // enum members to check against.
        }
    }
    Ok(())
}

/// The name JavaScript's `typeof` would give a JSON value, so that the "got X"
/// half of a message matches the reference.
fn js_typeof(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::String(_) => "string",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Bool(_) => "boolean",
        // `typeof null` is `"object"` in JavaScript, and an unrepresentable
        // number arrives here as JSON null.
        serde_json::Value::Null => "object",
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => "object",
    }
}

/// Match `/^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}:\d{2}(\.\d{3})?Z?)?$/`.
///
/// This is a **shape** check, not a calendar check: `2024-13-99` passes here
/// because it passes in the bundle. Hand-rolled to avoid a regex dependency for
/// one pattern.
fn is_iso_date_shape(text: &str) -> bool {
    let bytes = text.as_bytes();
    // YYYY-MM-DD is exactly ten characters.
    if bytes.len() < 10 {
        return false;
    }
    let digits = |range: std::ops::Range<usize>| bytes[range].iter().all(u8::is_ascii_digit);
    if !digits(0..4) || bytes[4] != b'-' || !digits(5..7) || bytes[7] != b'-' || !digits(8..10) {
        return false;
    }
    if bytes.len() == 10 {
        return true;
    }
    // The optional time part: THH:MM:SS
    if bytes[10] != b'T' || bytes.len() < 19 {
        return false;
    }
    if !digits(11..13) || bytes[13] != b':' || !digits(14..16) || bytes[16] != b':' {
        return false;
    }
    if !digits(17..19) {
        return false;
    }
    let mut i = 19;
    // The optional milliseconds: exactly three digits after the dot.
    if bytes.get(i) == Some(&b'.') {
        if bytes.len() < i + 4 || !digits(i + 1..i + 4) {
            return false;
        }
        i += 4;
    }
    // The optional trailing Z.
    if bytes.get(i) == Some(&b'Z') {
        i += 1;
    }
    i == bytes.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_query;
    use serde_json::json;

    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("age", ColumnType::Number),
            ColumnDef::new("name", ColumnType::String),
            ColumnDef::new("active", ColumnType::Boolean),
            ColumnDef::new("created", ColumnType::Date),
            ColumnDef::new("kind", ColumnType::Enum),
        ]
    }

    /// The single message a query produces, or a panic if it produced none.
    fn message(query: &str) -> String {
        let result = parse_query(query, &columns());
        let errors = result.errors();
        assert_eq!(errors.len(), 1, "{query}: {errors:?}");
        errors[0].message.clone()
    }

    fn is_valid(query: &str) -> bool {
        !parse_query(query, &columns()).has_errors()
    }

    #[test]
    fn unknown_column_is_reported() {
        assert_eq!(message("[nope] = 1"), "Column 'nope' does not exist");
        assert_eq!(message("[ s ] = \"x\""), "Column ' s ' does not exist");
        // A non-existent column stops validation of that condition, so the
        // operator/type mismatch that would otherwise follow is not also
        // reported. The operand has to be well-typed for the *grammar* first:
        // `[nope] contains 1` fails to parse, so validation never runs on it.
        assert_eq!(
            message("[nope] contains \"x\""),
            "Column 'nope' does not exist"
        );
        assert_eq!(message("[nope] is blank"), "Column 'nope' does not exist");
        assert_eq!(
            message("[nope] contains 1"),
            "mismatched input '1' expecting STRING"
        );
    }

    #[test]
    fn semantic_errors_carry_zero_positions() {
        let result = parse_query("[nope] = 1", &columns());
        let e = &result.errors()[0];
        assert_eq!((e.start, e.end), (0, 0));
        assert_eq!(e.severity, crate::parser::ErrorSeverity::Error);
    }

    #[test]
    fn incompatible_operators_on_string() {
        assert_eq!(
            message("[name] > 1"),
            "Operator 'greater than' is not compatible with type 'string'"
        );
        assert_eq!(
            message("[name] < 1"),
            "Operator 'less than' is not compatible with type 'string'"
        );
        assert_eq!(
            message("[name] >= 1"),
            "Operator 'greater than or equal' is not compatible with type 'string'"
        );
        assert_eq!(
            message("[name] <= 1"),
            "Operator 'less than or equal' is not compatible with type 'string'"
        );
    }

    #[test]
    fn compatible_operators_on_string() {
        assert!(is_valid("[name] = \"a\""));
        assert!(is_valid("[name] contains \"a\""));
        assert!(is_valid("[name] starts with \"a\""));
        assert!(is_valid("[name] ends with \"a\""));
    }

    #[test]
    fn incompatible_operators_on_number() {
        assert_eq!(
            message("[age] contains \"x\""),
            "Operator 'contains' is not compatible with type 'number'"
        );
        assert_eq!(
            message("[age] starts with \"x\""),
            "Operator 'starts with' is not compatible with type 'number'"
        );
        assert_eq!(
            message("[age] ends with \"x\""),
            "Operator 'ends with' is not compatible with type 'number'"
        );
    }

    #[test]
    fn compatible_operators_on_number_and_date() {
        for op in ["=", ">", "<", ">=", "<="] {
            assert!(is_valid(&format!("[age] {op} 1")), "age {op}");
            assert!(
                is_valid(&format!("[created] {op} \"2024-01-01\"")),
                "created {op}"
            );
        }
    }

    #[test]
    fn incompatible_operators_on_boolean() {
        assert_eq!(
            message("[active] > 1"),
            "Operator 'greater than' is not compatible with type 'boolean'"
        );
        assert_eq!(
            message("[active] contains \"x\""),
            "Operator 'contains' is not compatible with type 'boolean'"
        );
        assert!(is_valid("[active] = true"));
        assert!(is_valid("[active] = false"));
    }

    #[test]
    fn incompatible_operators_on_enum() {
        assert_eq!(
            message("[kind] contains \"a\""),
            "Operator 'contains' is not compatible with type 'enum'"
        );
        assert_eq!(
            message("[kind] > 1"),
            "Operator 'greater than' is not compatible with type 'enum'"
        );
        assert!(is_valid("[kind] = \"a\""));
    }

    #[test]
    fn incompatible_operators_on_date() {
        assert_eq!(
            message("[created] contains \"a\""),
            "Operator 'contains' is not compatible with type 'date'"
        );
    }

    #[test]
    fn blank_operators_are_string_date_and_enum_only() {
        assert!(is_valid("[name] is blank"));
        assert!(is_valid("[name] is not blank"));
        assert!(is_valid("[created] is blank"));
        assert!(is_valid("[kind] is blank"));
        assert_eq!(
            message("[active] is blank"),
            "Operator 'is blank' is not compatible with type 'boolean'"
        );
        assert_eq!(
            message("[active] is not blank"),
            "Operator 'is not blank' is not compatible with type 'boolean'"
        );
        assert_eq!(
            message("[age] is blank"),
            "Operator 'is blank' is not compatible with type 'number'"
        );
    }

    #[test]
    fn wrong_value_types_are_reported() {
        assert_eq!(
            message("[age] > \"x\""),
            "Expected number for column 'age', got string"
        );
        assert_eq!(
            message("[age] = true"),
            "Expected number for column 'age', got boolean"
        );
        assert_eq!(
            message("[name] = 1"),
            "Expected string for column 'name', got number"
        );
        assert_eq!(
            message("[name] = true"),
            "Expected string for column 'name', got boolean"
        );
        assert_eq!(
            message("[active] = \"x\""),
            "Expected boolean for column 'active', got string"
        );
        assert_eq!(
            message("[active] = 1"),
            "Expected boolean for column 'active', got number"
        );
        assert_eq!(
            message("[created] > 1"),
            "Expected date string for column 'created', got number"
        );
        assert_eq!(
            message("[kind] = 1"),
            "Expected string for enum column 'kind', got number"
        );
    }

    #[test]
    fn date_check_is_shape_only() {
        // Nonsense month and day, accepted because the reference accepts them.
        assert!(is_valid("[created] > \"2024-13-99\""));
        assert!(is_valid("[created] > \"0000-00-00\""));
    }

    #[test]
    fn date_shapes_that_are_accepted() {
        for value in [
            "2024-01-01",
            "2024-01-01T10:20:30",
            "2024-01-01T10:20:30Z",
            "2024-01-01T10:20:30.123",
            "2024-01-01T10:20:30.123Z",
        ] {
            assert!(is_iso_date_shape(value), "{value}");
            assert!(is_valid(&format!("[created] > \"{value}\"")), "{value}");
        }
    }

    #[test]
    fn date_shapes_that_are_rejected() {
        for value in [
            "2024-1-1",
            "2024-01-01T10:20:30.12",
            "2024-01-01T10:20:30.1234",
            "2024-01-01T10:20",
            "2024-01-01 10:20:30",
            "2024-01-01T",
            "2024-01-01Z",
            "24-01-01",
            "",
            "x",
            "2024-01-01T10:20:30.123ZZ",
            "2024-01-01T10:20:30z",
        ] {
            assert!(!is_iso_date_shape(value), "{value}");
        }
        assert_eq!(
            message("[created] > \"2024-1-1\""),
            "Invalid date format for column 'created'. Expected YYYY-MM-DD or ISO datetime"
        );
    }

    #[test]
    fn a_comparison_with_no_value_is_reported() {
        // The grammar cannot produce this, but a hand-built AST can.
        let group = FilterGroup::and(vec![Filter::condition(
            "age",
            FilterFunction::GreaterThan,
            vec![],
        )]);
        let errors = validate_filter_group(&group, &columns());
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Operator 'greater than' requires a value"
        );
    }

    #[test]
    fn nested_groups_are_validated() {
        let result = parse_query("([nope] = 1 OR [age] = 2) AND [name] = 3", &columns());
        let messages: Vec<_> = result.errors().iter().map(|e| e.message.clone()).collect();
        assert_eq!(
            messages,
            vec![
                "Column 'nope' does not exist".to_string(),
                "Expected string for column 'name', got number".to_string(),
            ]
        );
    }

    #[test]
    fn an_empty_group_validates_clean() {
        assert!(validate_filter_group(&FilterGroup::default(), &columns()).is_empty());
    }

    #[test]
    fn a_filter_with_neither_side_is_ignored() {
        let group = FilterGroup::and(vec![Filter::default()]);
        assert!(validate_filter_group(&group, &columns()).is_empty());
    }

    #[test]
    fn all_args_are_checked() {
        let group = FilterGroup::and(vec![Filter::condition(
            "age",
            FilterFunction::Equals,
            vec![json!(1), json!("x"), json!(true)],
        )]);
        let errors = validate_filter_group(&group, &columns());
        assert_eq!(errors.len(), 2);
        assert_eq!(
            errors[0].message,
            "Expected number for column 'age', got string"
        );
        assert_eq!(
            errors[1].message,
            "Expected number for column 'age', got boolean"
        );
    }

    #[test]
    fn a_null_arg_reports_the_javascript_typeof() {
        let group = FilterGroup::and(vec![Filter::condition(
            "age",
            FilterFunction::Equals,
            vec![serde_json::Value::Null],
        )]);
        let errors = validate_filter_group(&group, &columns());
        assert_eq!(
            errors[0].message,
            "Expected number for column 'age', got object"
        );
    }

    #[test]
    fn compatibility_helpers_cover_every_pair() {
        use FilterFunction::*;
        let all = [
            Equals,
            GreaterThan,
            LessThan,
            GreaterThanOrEqual,
            LessThanOrEqual,
            Contains,
            StartsWith,
            EndsWith,
        ];
        let types = [
            ColumnType::String,
            ColumnType::Number,
            ColumnType::Boolean,
            ColumnType::Date,
            ColumnType::Enum,
        ];
        // Equality is the one operator every type accepts.
        for t in types {
            assert!(is_operator_compatible(t, Equals), "{t:?}");
        }
        // And every pair has a definite answer, so no combination is unhandled.
        for t in types {
            for f in all {
                let _ = is_operator_compatible(t, f);
            }
        }
        assert!(is_blank_operator_compatible(ColumnType::String));
        assert!(is_blank_operator_compatible(ColumnType::Date));
        assert!(is_blank_operator_compatible(ColumnType::Enum));
        assert!(!is_blank_operator_compatible(ColumnType::Number));
        assert!(!is_blank_operator_compatible(ColumnType::Boolean));
    }

    #[test]
    fn normalized_backend_types_validate_like_their_targets() {
        let cols = vec![
            ColumnDef::normalized("nAge", "INT32"),
            ColumnDef::normalized("tName", "TEXT"),
            ColumnDef::normalized("weird", "GUID"),
        ];
        assert!(!parse_query("[nAge] > 1", &cols).has_errors());
        assert!(!parse_query("[tName] contains \"a\"", &cols).has_errors());
        // An unknown backend type falls back to string, so `contains` is fine
        // and `>` is not.
        assert!(!parse_query("[weird] contains \"a\"", &cols).has_errors());
        let result = parse_query("[weird] > 1", &cols);
        assert_eq!(
            result.errors()[0].message,
            "Operator 'greater than' is not compatible with type 'string'"
        );
    }
}
