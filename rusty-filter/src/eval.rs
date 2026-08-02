//! Evaluating a filter against row data.
//!
//! A port of `FilterEvaluator` and `Comparators`. Rows are
//! [`serde_json::Value`] objects, which is what `DataTable` already stores, so
//! nothing needs converting.
//!
//! Three behaviours are worth knowing before reading the code:
//!
//! * **An empty group is `true`, whatever its operator.** The reference returns
//!   early on `filters.length === 0` before it looks at `op`, so an empty `OR`
//!   group is vacuously true just like an empty `AND` group. Keep it that way:
//!   the empty query parses to an empty group and must match every row.
//! * **The column type decides which operators can match.** `contains` on a
//!   non-string column is `false`, not a string coercion; the orderings on
//!   anything but a number or a date are `false`. Because [`ColumnDef`] stores a
//!   normalized [`ColumnType`], a column declared `INT32` compares as a number
//!   here, whereas the reference — which switches on the raw type name —
//!   silently returns `false`. That divergence is deliberate.
//! * **A non-object row has no columns.** The reference reads `row[column]`,
//!   which throws for `null` and is caught into a blanket `false`; here every
//!   column of a non-object row reads as missing, so `isBlank` matches it.

use std::collections::HashMap;

use crate::ast::{Condition, Filter, FilterFunction, FilterGroup, LogicalOp};
use crate::column::{ColumnDef, ColumnType};

/// The recursion limit from `FilterEvaluator`. Exceeding it fails the whole
/// evaluation rather than the offending branch, as the reference's throw and
/// blanket `catch` do.
const MAX_RECURSION_DEPTH: usize = 100;

/// A name-to-column lookup. On a duplicate name the first definition wins,
/// matching [`crate::column::find_column`] and so keeping validation and
/// evaluation in agreement.
type ColumnMap<'a> = HashMap<&'a str, &'a ColumnDef>;

fn column_map<'a>(columns: &'a [ColumnDef]) -> ColumnMap<'a> {
    let mut map = ColumnMap::with_capacity(columns.len());
    for column in columns {
        map.entry(column.name.as_str()).or_insert(column);
    }
    map
}

/// Whether `row` matches `filter`.
///
/// Returns `false` if the filter nests deeper than 100 groups.
pub fn evaluate(filter: &FilterGroup, row: &serde_json::Value, columns: &[ColumnDef]) -> bool {
    eval_group(filter, row, &column_map(columns), 0).unwrap_or(false)
}

/// Keep only the rows matching `filter`, preserving their order.
pub fn retain_matching(
    filter: &FilterGroup,
    rows: Vec<serde_json::Value>,
    columns: &[ColumnDef],
) -> Vec<serde_json::Value> {
    let map = column_map(columns);
    rows.into_iter()
        .filter(|row| eval_group(filter, row, &map, 0).unwrap_or(false))
        .collect()
}

/// How many of `rows` match `filter`, without building a new collection.
pub fn count_matches(
    filter: &FilterGroup,
    rows: &[serde_json::Value],
    columns: &[ColumnDef],
) -> usize {
    let map = column_map(columns);
    rows.iter()
        .filter(|row| eval_group(filter, row, &map, 0).unwrap_or(false))
        .count()
}

/// `None` stands for the reference's "maximum recursion depth exceeded" throw,
/// which its caller turns into `false` for the entire filter.
fn eval_group(
    group: &FilterGroup,
    row: &serde_json::Value,
    columns: &ColumnMap<'_>,
    depth: usize,
) -> Option<bool> {
    if depth > MAX_RECURSION_DEPTH {
        return None;
    }
    // Vacuous truth, checked before `op` is examined.
    if group.filters.is_empty() {
        return Some(true);
    }
    match group.op {
        LogicalOp::And => {
            for filter in &group.filters {
                if !eval_filter(filter, row, columns, depth + 1)? {
                    return Some(false);
                }
            }
            Some(true)
        }
        LogicalOp::Or => {
            for filter in &group.filters {
                if eval_filter(filter, row, columns, depth + 1)? {
                    return Some(true);
                }
            }
            Some(false)
        }
    }
}

fn eval_filter(
    filter: &Filter,
    row: &serde_json::Value,
    columns: &ColumnMap<'_>,
    depth: usize,
) -> Option<bool> {
    // A condition wins over a group when both are set, and a filter with
    // neither never matches — even negated, since the reference returns before
    // it applies negation.
    let result = if let Some(condition) = &filter.condition {
        eval_condition(condition, row, columns)
    } else if let Some(group) = &filter.group {
        // The nested group keeps this depth rather than incrementing again.
        eval_group(group, row, columns, depth)?
    } else {
        return Some(false);
    };
    Some(result != filter.is_negated())
}

fn eval_condition(condition: &Condition, row: &serde_json::Value, columns: &ColumnMap<'_>) -> bool {
    let Some(column) = columns.get(condition.column.as_str()) else {
        // A condition on an unknown column never matches.
        return false;
    };
    let value = row.get(&condition.column);
    apply(
        condition.function,
        value,
        &condition.args,
        column.column_type,
    )
}

/// Whether `value` — `None` when the row has no such key — satisfies
/// `function` against `args` for a column of `column_type`.
pub fn apply(
    function: FilterFunction,
    value: Option<&serde_json::Value>,
    args: &[serde_json::Value],
    column_type: ColumnType,
) -> bool {
    // The blank operators ignore their arguments and are the only ones a
    // missing value can satisfy.
    match function {
        FilterFunction::IsBlank => return is_blank(value, column_type),
        FilterFunction::IsNotBlank => return !is_blank(value, column_type),
        _ => {}
    }

    let (Some(value), Some(arg)) = (non_null(value), non_null(args.first())) else {
        return false;
    };

    match function {
        FilterFunction::Equals => strict_equals(value, arg),
        FilterFunction::GreaterThan
        | FilterFunction::LessThan
        | FilterFunction::GreaterThanOrEqual
        | FilterFunction::LessThanOrEqual => compare(function, value, arg, column_type),
        FilterFunction::Contains | FilterFunction::StartsWith | FilterFunction::EndsWith => {
            if column_type != ColumnType::String {
                return false;
            }
            let (Some(haystack), Some(needle)) = (value.as_str(), arg.as_str()) else {
                return false;
            };
            // Case-sensitive on purpose: the reference does not fold case.
            match function {
                FilterFunction::Contains => haystack.contains(needle),
                FilterFunction::StartsWith => haystack.starts_with(needle),
                _ => haystack.ends_with(needle),
            }
        }
        FilterFunction::IsBlank | FilterFunction::IsNotBlank => unreachable!("handled above"),
    }
}

/// A missing key and a JSON `null` are both the reference's "nullish".
fn non_null(value: Option<&serde_json::Value>) -> Option<&serde_json::Value> {
    match value {
        Some(serde_json::Value::Null) | None => None,
        other => other,
    }
}

fn is_blank(value: Option<&serde_json::Value>, column_type: ColumnType) -> bool {
    match non_null(value) {
        None => true,
        // The empty string counts as blank for string columns only.
        Some(value) => column_type == ColumnType::String && value.as_str() == Some(""),
    }
}

/// JavaScript `===` on two JSON values.
///
/// Strict, so no coercion: the number `1` never equals the string `"1"`. Numbers
/// are compared by value rather than by JSON representation, because `1` and
/// `1.0` are the same number in JavaScript and a row loaded from a database may
/// carry either.
fn strict_equals(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (a, b) {
        (serde_json::Value::Number(x), serde_json::Value::Number(y)) => x.as_f64() == y.as_f64(),
        // Arrays and objects compare by identity in JavaScript, so two distinct
        // ones are never equal however alike they look.
        (serde_json::Value::Array(_), _)
        | (_, serde_json::Value::Array(_))
        | (serde_json::Value::Object(_), _)
        | (_, serde_json::Value::Object(_)) => false,
        _ => a == b,
    }
}

fn compare(
    function: FilterFunction,
    value: &serde_json::Value,
    arg: &serde_json::Value,
    column_type: ColumnType,
) -> bool {
    let ordering = match column_type {
        ColumnType::Number => {
            let (Some(a), Some(b)) = (value.as_f64(), arg.as_f64()) else {
                return false;
            };
            a.partial_cmp(&b)
        }
        ColumnType::Date => {
            let (Some(a), Some(b)) = (value.as_str(), arg.as_str()) else {
                return false;
            };
            let (Some(a), Some(b)) = (parse_iso_millis(a), parse_iso_millis(b)) else {
                // An unparseable date is the reference's `NaN`, which fails
                // every comparison.
                return false;
            };
            Some(a.cmp(&b))
        }
        // The orderings are not supported for other types.
        ColumnType::String | ColumnType::Boolean | ColumnType::Enum => return false,
    };
    let Some(ordering) = ordering else {
        return false;
    };
    match function {
        FilterFunction::GreaterThan => ordering.is_gt(),
        FilterFunction::LessThan => ordering.is_lt(),
        FilterFunction::GreaterThanOrEqual => ordering.is_ge(),
        FilterFunction::LessThanOrEqual => ordering.is_le(),
        _ => false,
    }
}

/// Parse an ISO 8601 date or datetime into milliseconds since the Unix epoch,
/// returning `None` where `Date.parse` would return `NaN`.
///
/// This follows ECMAScript's Date Time String Format, which is what the
/// reference's `new Date(...)` reaches for: the field ranges are checked
/// (month 1–12, day 1–31, hour 0–24, minute and second 0–59, and hour 24 only at
/// exactly `24:00:00`), then the arithmetic is allowed to roll over, so
/// `2024-02-30` is 1 March and `2024-01-01T24:00:00` is 2 January — while
/// `2024-13-01` and `2024-01-32` are unparseable.
///
/// **One deliberate divergence.** A datetime with no trailing `Z` is read as
/// UTC, whereas the browser reads it in the machine's local zone. Following the
/// browser would make server-side filtering depend on the server's timezone and
/// disagree with the same filter run in the client, so a date-only value and a
/// naked datetime compare equal here — `2024-01-01T00:00:00 > 2024-01-01` is
/// false in both implementations, but for different reasons.
fn parse_iso_millis(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    let digits = |start: usize, len: usize| -> Option<i64> {
        let slice = bytes.get(start..start + len)?;
        if !slice.iter().all(u8::is_ascii_digit) {
            return None;
        }
        std::str::from_utf8(slice).ok()?.parse().ok()
    };

    let year = digits(0, 4)?;
    if bytes.get(4) != Some(&b'-') {
        return None;
    }
    let month = digits(5, 2)?;
    if bytes.get(7) != Some(&b'-') {
        return None;
    }
    let day = digits(8, 2)?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let mut hour = 0;
    let mut minute = 0;
    let mut second = 0;
    let mut milli = 0;
    let mut end = 10;
    if bytes.len() > 10 {
        if bytes[10] != b'T' {
            return None;
        }
        hour = digits(11, 2)?;
        if bytes.get(13) != Some(&b':') {
            return None;
        }
        minute = digits(14, 2)?;
        if bytes.get(16) != Some(&b':') {
            return None;
        }
        second = digits(17, 2)?;
        end = 19;
        if bytes.get(end) == Some(&b'.') {
            milli = digits(end + 1, 3)?;
            end += 4;
        }
        if !(0..=24).contains(&hour) || minute > 59 || second > 59 {
            return None;
        }
        // Hour 24 names midnight ending the day, so nothing may follow it.
        if hour == 24 && (minute, second, milli) != (0, 0, 0) {
            return None;
        }
        if bytes.get(end) == Some(&b'Z') {
            end += 1;
        }
    }
    if end != bytes.len() {
        return None;
    }

    let days = days_from_civil(year, month, day);
    Some(((days * 24 + hour) * 60 + minute) * 60_000 + second * 1_000 + milli)
}

/// Days from 1970-01-01 to `year-month-day` in the proleptic Gregorian
/// calendar. A day past the end of its month rolls into the next one, as
/// ECMAScript's `MakeDay` does.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    // Shift so that the leap day falls at the end of the 400-year cycle.
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let year_of_era = y - era * 400;
    let month_shifted = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_shifted + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_query_unchecked;
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

    fn matches(query: &str, row: serde_json::Value) -> bool {
        let group = parse_query_unchecked(query).expect(query);
        evaluate(&group, &row, &columns())
    }

    #[test]
    fn the_probed_rows_behave_as_measured() {
        let query = "[age] > 1 AND [name] contains \"a\"";
        assert!(matches(query, json!({"age": 2, "name": "abc"})));
        assert!(!matches(query, json!({"age": 0, "name": "abc"})));
        assert!(!matches(query, json!({"age": 2, "name": "xyz"})));
        assert!(!matches(query, json!({"age": null, "name": "abc"})));
    }

    #[test]
    fn or_needs_only_one_arm() {
        let query = "[age] > 10 OR [name] = \"a\"";
        assert!(matches(query, json!({"age": 11, "name": "z"})));
        assert!(matches(query, json!({"age": 0, "name": "a"})));
        assert!(!matches(query, json!({"age": 0, "name": "z"})));
    }

    #[test]
    fn an_empty_group_is_true_whatever_its_operator() {
        let row = json!({"age": 1});
        // This is the divergence from the plan's table: an empty OR group is
        // true in the bundle, not false.
        assert!(evaluate(&FilterGroup::and(vec![]), &row, &columns()));
        assert!(evaluate(&FilterGroup::or(vec![]), &row, &columns()));
        // And the empty query parses to exactly that group.
        assert!(matches("", row.clone()));
        assert!(matches("   ", row));
    }

    #[test]
    fn an_empty_group_matches_even_a_non_object_row() {
        assert!(evaluate(&FilterGroup::default(), &json!(null), &columns()));
        assert!(evaluate(&FilterGroup::default(), &json!(7), &columns()));
    }

    #[test]
    fn negation_inverts_the_filter() {
        assert!(matches("NOT [age] > 1", json!({"age": 0})));
        assert!(!matches("NOT [age] > 1", json!({"age": 2})));
        assert!(matches("[name] not contains \"a\"", json!({"name": "xyz"})));
        assert!(!matches(
            "[name] not contains \"a\"",
            json!({"name": "abc"})
        ));
        // A false `negate` key must not invert anything.
        let group = FilterGroup::and(vec![Filter::condition(
            "age",
            FilterFunction::GreaterThan,
            vec![json!(1)],
        )
        .negated(false)]);
        assert!(evaluate(&group, &json!({"age": 2}), &columns()));
    }

    #[test]
    fn negation_applies_to_groups_too() {
        assert!(matches(
            "NOT ([age] > 1 AND [name] = \"a\")",
            json!({"age": 2, "name": "b"})
        ));
        assert!(!matches(
            "NOT ([age] > 1 AND [name] = \"a\")",
            json!({"age": 2, "name": "a"})
        ));
    }

    #[test]
    fn a_null_value_fails_everything_but_is_blank() {
        for query in [
            "[name] = \"a\"",
            "[name] contains \"a\"",
            "[name] starts with \"a\"",
            "[name] ends with \"a\"",
            "[name] is not blank",
        ] {
            assert!(!matches(query, json!({"name": null})), "{query}");
            assert!(!matches(query, json!({})), "{query} missing");
        }
        assert!(matches("[name] is blank", json!({"name": null})));
        assert!(matches("[name] is blank", json!({})));
    }

    #[test]
    fn a_missing_column_definition_never_matches() {
        let group = parse_query_unchecked("[nope] is blank").unwrap();
        assert!(!evaluate(&group, &json!({"nope": null}), &columns()));
    }

    #[test]
    fn is_blank_covers_the_empty_string_for_strings_only() {
        assert!(matches("[name] is blank", json!({"name": ""})));
        assert!(!matches("[name] is blank", json!({"name": "a"})));
        assert!(!matches("[name] is not blank", json!({"name": ""})));
        // An empty string in a date or enum column is not blank.
        assert!(!matches("[created] is blank", json!({"created": ""})));
        assert!(!matches("[kind] is blank", json!({"kind": ""})));
        assert!(matches("[kind] is not blank", json!({"kind": ""})));
    }

    #[test]
    fn equals_is_strict() {
        assert!(!matches("[name] = \"1\"", json!({"name": 1})));
        assert!(matches("[name] = \"1\"", json!({"name": "1"})));
        assert!(!matches("[age] = 1", json!({"age": "1"})));
        assert!(!matches("[age] = 1", json!({"age": true})));
        assert!(!matches("[active] = true", json!({"active": 1})));
        assert!(matches("[active] = true", json!({"active": true})));
        assert!(matches("[active] = false", json!({"active": false})));
        assert!(!matches("[active] = false", json!({"active": true})));
    }

    #[test]
    fn equals_compares_numbers_by_value_not_representation() {
        // A row loaded from a database may hold an integer where the query
        // holds a float, and JavaScript would call those equal.
        assert!(matches("[age] = 1", json!({"age": 1})));
        assert!(matches("[age] = 1", json!({"age": 1.0})));
        assert!(matches("[age] = 1.0", json!({"age": 1})));
        assert!(matches("[age] = 1.5", json!({"age": 1.5})));
        assert!(!matches("[age] = 1.5", json!({"age": 1.6})));
    }

    #[test]
    fn equals_never_matches_a_composite_value() {
        let group = FilterGroup::and(vec![Filter::condition(
            "name",
            FilterFunction::Equals,
            vec![json!([1])],
        )]);
        assert!(!evaluate(&group, &json!({"name": [1]}), &columns()));
        let group = FilterGroup::and(vec![Filter::condition(
            "name",
            FilterFunction::Equals,
            vec![json!({"a": 1})],
        )]);
        assert!(!evaluate(&group, &json!({"name": {"a": 1}}), &columns()));
    }

    #[test]
    fn number_orderings() {
        assert!(matches("[age] > 1", json!({"age": 1.5})));
        assert!(!matches("[age] > 1", json!({"age": 1})));
        assert!(matches("[age] >= 1", json!({"age": 1})));
        assert!(matches("[age] < 1", json!({"age": 0.5})));
        assert!(!matches("[age] < 1", json!({"age": 1})));
        assert!(matches("[age] <= 1", json!({"age": 1})));
        assert!(matches("[age] > -1", json!({"age": 0})));
        // A non-numeric value in a numeric column cannot be ordered.
        assert!(!matches("[age] > 1", json!({"age": "5"})));
        assert!(!matches("[age] > 1", json!({"age": true})));
    }

    #[test]
    fn orderings_are_unsupported_for_string_boolean_and_enum() {
        // The grammar and validator both reject these, but a hand-built AST
        // can carry them, and then they must not match.
        for column in ["name", "active", "kind"] {
            let group = FilterGroup::and(vec![Filter::condition(
                column,
                FilterFunction::GreaterThan,
                vec![json!("a")],
            )]);
            let row = json!({"name": "b", "active": true, "kind": "b"});
            assert!(!evaluate(&group, &row, &columns()), "{column}");
        }
    }

    #[test]
    fn contains_and_friends_are_case_sensitive() {
        assert!(matches("[name] contains \"bc\"", json!({"name": "abcd"})));
        assert!(!matches("[name] contains \"BC\"", json!({"name": "abcd"})));
        assert!(matches(
            "[name] starts with \"ab\"",
            json!({"name": "abcd"})
        ));
        assert!(!matches(
            "[name] starts with \"AB\"",
            json!({"name": "abcd"})
        ));
        assert!(matches("[name] ends with \"cd\"", json!({"name": "abcd"})));
        assert!(!matches("[name] ends with \"CD\"", json!({"name": "abcd"})));
        // The empty needle is contained in everything.
        assert!(matches("[name] contains \"\"", json!({"name": "abcd"})));
    }

    #[test]
    fn text_operations_need_a_string_column() {
        // `[kind]` is an enum, so `contains` cannot match however alike the
        // values look.
        let group = FilterGroup::and(vec![Filter::condition(
            "kind",
            FilterFunction::Contains,
            vec![json!("b")],
        )]);
        assert!(!evaluate(&group, &json!({"kind": "abc"}), &columns()));
    }

    #[test]
    fn date_orderings() {
        assert!(matches(
            "[created] > \"2024-01-01\"",
            json!({"created": "2024-06-01"})
        ));
        assert!(!matches(
            "[created] > \"2024-01-01\"",
            json!({"created": "2023-06-01"})
        ));
        assert!(matches(
            "[created] >= \"2024-01-01\"",
            json!({"created": "2024-01-01"})
        ));
        assert!(matches(
            "[created] < \"2024-01-01T12:00:00Z\"",
            json!({"created": "2024-01-01T11:59:59Z"})
        ));
        // Different shapes still compare on the instant they name.
        assert!(matches(
            "[created] <= \"2024-01-01\"",
            json!({"created": "2024-01-01T00:00:00Z"})
        ));
        assert!(matches(
            "[created] >= \"2024-01-01\"",
            json!({"created": "2024-01-01T00:00:00Z"})
        ));
    }

    #[test]
    fn a_naked_datetime_does_not_exceed_the_bare_date() {
        // The measured browser answer, reached here without depending on the
        // machine's timezone.
        assert!(!matches(
            "[created] > \"2024-01-01\"",
            json!({"created": "2024-01-01T00:00:00"})
        ));
    }

    #[test]
    fn an_unparseable_date_fails_every_ordering() {
        for query in [
            "[created] > \"2024-13-99\"",
            "[created] < \"2024-13-99\"",
            "[created] >= \"2024-13-99\"",
            "[created] <= \"2024-13-99\"",
        ] {
            assert!(!matches(query, json!({"created": "2024-01-01"})), "{query}");
        }
        assert!(!matches(
            "[created] > \"2024-01-01\"",
            json!({"created": "not a date"})
        ));
        // A non-string value in a date column cannot be ordered either.
        assert!(!matches(
            "[created] > \"2024-01-01\"",
            json!({"created": 1})
        ));
    }

    #[test]
    fn iso_parsing_matches_the_javascript_engine() {
        // Values measured with `Date.parse` in Node on 2026-08-01.
        assert_eq!(parse_iso_millis("2024-01-01"), Some(1_704_067_200_000));
        assert_eq!(
            parse_iso_millis("2024-01-01T00:00:00Z"),
            Some(1_704_067_200_000)
        );
        assert_eq!(
            parse_iso_millis("2024-12-31T23:59:59.999Z"),
            Some(1_735_689_599_999)
        );
        assert_eq!(parse_iso_millis("0000-01-01"), Some(-62_167_219_200_000));
        assert_eq!(parse_iso_millis("9999-12-31"), Some(253_402_214_400_000));
        assert_eq!(parse_iso_millis("2024-02-29"), Some(1_709_164_800_000));
        // Out-of-range fields are unparseable, exactly as in the engine.
        for text in [
            "2024-13-01",
            "2024-00-01",
            "2024-01-32",
            "2024-01-00",
            "2024-01-01T25:00:00",
            "2024-01-01T23:60:00",
            "2024-01-01T23:59:60",
            "2024-01-01T24:00:01",
            "2024-01-01T24:59:59",
            "2024-1-1",
            "",
            "x",
            "2024-01-01 00:00:00",
            "2024-01-01T00:00",
            "2024-01-01T00:00:00.12",
            "2024-01-01T00:00:00ZZ",
        ] {
            assert_eq!(parse_iso_millis(text), None, "{text}");
        }
    }

    #[test]
    fn iso_parsing_rolls_over_like_make_day_and_make_time() {
        // Both measured in Node: an over-long month rolls into the next one,
        // and hour 24 names the following midnight.
        assert_eq!(
            parse_iso_millis("2024-02-30"),
            parse_iso_millis("2024-03-01")
        );
        assert_eq!(
            parse_iso_millis("2023-02-29"),
            parse_iso_millis("2023-03-01")
        );
        assert_eq!(
            parse_iso_millis("2024-04-31"),
            parse_iso_millis("2024-05-01")
        );
        assert_eq!(
            parse_iso_millis("2024-01-01T24:00:00Z"),
            parse_iso_millis("2024-01-02")
        );
    }

    #[test]
    fn days_from_civil_anchors_on_the_epoch() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(1970, 1, 2), 1);
        assert_eq!(days_from_civil(1969, 12, 31), -1);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(days_from_civil(1900, 3, 1), -25_508);
    }

    #[test]
    fn normalized_types_widen_what_can_match() {
        // The reference switches on the raw type name, so an `INT32` column
        // fails every ordering there. Here the type is normalized first.
        let cols = vec![
            ColumnDef::normalized("n", "INT32"),
            ColumnDef::normalized("t", "TEXT"),
        ];
        let group = parse_query_unchecked("[n] > 1 AND [t] contains \"a\"").unwrap();
        assert!(evaluate(&group, &json!({"n": 2, "t": "abc"}), &cols));
        assert!(!evaluate(&group, &json!({"n": 1, "t": "abc"}), &cols));
    }

    #[test]
    fn a_non_object_row_has_no_columns() {
        let group = parse_query_unchecked("[name] is blank").unwrap();
        assert!(evaluate(&group, &json!(null), &columns()));
        assert!(evaluate(&group, &json!([1, 2]), &columns()));
        let group = parse_query_unchecked("[name] = \"a\"").unwrap();
        assert!(!evaluate(&group, &json!(null), &columns()));
    }

    #[test]
    fn a_filter_with_neither_side_never_matches() {
        let group = FilterGroup::and(vec![Filter::default()]);
        assert!(!evaluate(&group, &json!({}), &columns()));
        // Negation does not rescue it: the reference returns before negating.
        let group = FilterGroup::and(vec![Filter::default().negated(true)]);
        assert!(!evaluate(&group, &json!({}), &columns()));
    }

    #[test]
    fn a_condition_wins_over_a_group_on_the_same_filter() {
        let filter = Filter {
            condition: Some(crate::ast::Condition::new(
                "age",
                FilterFunction::GreaterThan,
                vec![json!(1)],
            )),
            group: Some(FilterGroup::and(vec![Filter::condition(
                "age",
                FilterFunction::LessThan,
                vec![json!(0)],
            )])),
            negate: None,
        };
        let group = FilterGroup::and(vec![filter]);
        assert!(evaluate(&group, &json!({"age": 2}), &columns()));
    }

    #[test]
    fn nesting_past_the_depth_limit_fails_the_whole_filter() {
        let deepest = FilterGroup::and(vec![Filter::condition(
            "age",
            FilterFunction::GreaterThan,
            vec![json!(1)],
        )]);
        let nest = |depth: usize| {
            let mut group = deepest.clone();
            for _ in 0..depth {
                group = FilterGroup::and(vec![Filter::from_group(group)]);
            }
            group
        };
        let row = json!({"age": 2});
        assert!(evaluate(&nest(99), &row, &columns()));
        assert!(evaluate(&nest(100), &row, &columns()));
        assert!(!evaluate(&nest(101), &row, &columns()));
    }

    #[test]
    fn retain_matching_keeps_order() {
        let group = parse_query_unchecked("[age] > 1").unwrap();
        let rows = vec![
            json!({"age": 3, "name": "c"}),
            json!({"age": 1, "name": "a"}),
            json!({"age": 2, "name": "b"}),
        ];
        let kept = retain_matching(&group, rows.clone(), &columns());
        assert_eq!(kept, vec![rows[0].clone(), rows[2].clone()]);
        assert_eq!(count_matches(&group, &rows, &columns()), 2);
    }

    #[test]
    fn retain_matching_on_an_empty_filter_keeps_everything() {
        let rows = vec![json!({"age": 1}), json!({"age": 2})];
        let kept = retain_matching(&FilterGroup::default(), rows.clone(), &columns());
        assert_eq!(kept, rows);
        assert_eq!(count_matches(&FilterGroup::default(), &rows, &columns()), 2);
    }

    #[test]
    fn duplicate_column_names_resolve_to_the_first() {
        let cols = vec![
            ColumnDef::new("x", ColumnType::Number),
            ColumnDef::new("x", ColumnType::String),
        ];
        let group = FilterGroup::and(vec![Filter::condition(
            "x",
            FilterFunction::GreaterThan,
            vec![json!(1)],
        )]);
        assert!(evaluate(&group, &json!({"x": 2}), &cols));
    }

    #[test]
    fn apply_is_usable_on_its_own() {
        assert!(apply(
            FilterFunction::Contains,
            Some(&json!("abc")),
            &[json!("b")],
            ColumnType::String
        ));
        assert!(!apply(
            FilterFunction::Contains,
            None,
            &[json!("b")],
            ColumnType::String
        ));
        // No argument means nothing to compare against.
        assert!(!apply(
            FilterFunction::Equals,
            Some(&json!("a")),
            &[],
            ColumnType::String
        ));
        assert!(apply(
            FilterFunction::IsBlank,
            None,
            &[],
            ColumnType::String
        ));
        // Only the first argument is read.
        assert!(apply(
            FilterFunction::Equals,
            Some(&json!("a")),
            &[json!("a"), json!("b")],
            ColumnType::String
        ));
    }
}
