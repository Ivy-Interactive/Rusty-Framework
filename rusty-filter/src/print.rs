//! Printing an AST back to a query string.
//!
//! A port of `ASTPrinter` in canonical mode only — the non-canonical option
//! combinations have no caller in this repo, so the options struct is not
//! reproduced.
//!
//! The canonical spelling is deliberately asymmetric: `=` prints as the word
//! `equals` while the four orderings stay symbolic. That is what the bundle
//! does, and changing it would make the two sides disagree on cache keys.

use crate::ast::{Condition, Filter, FilterFunction, FilterGroup, LogicalOp};

/// Print `group` as a canonical query string.
///
/// An empty group prints as the empty string. Output is stable for a given AST
/// but is not always re-parseable to the same *text*: `[name] not contains "a"`
/// prints as `NOT ([name] contains "a")`, which parses back to the same AST.
pub fn to_query_string(group: &FilterGroup) -> String {
    if group.filters.is_empty() {
        return String::new();
    }
    print_group(group, false)
}

/// The canonical string of `group`, for use as a cache key.
///
/// This is [`to_query_string`] under a name that says what it is for. Two ASTs
/// that filter identically produce the same key, and the frontend's
/// `formatQuery` produces that same key for the same AST.
pub fn canonical_key(group: &FilterGroup) -> String {
    to_query_string(group)
}

fn print_group(group: &FilterGroup, nested: bool) -> String {
    if group.filters.is_empty() {
        return String::new();
    }
    let separator = match group.op {
        LogicalOp::And => " AND ",
        LogicalOp::Or => " OR ",
    };
    let joined = group
        .filters
        .iter()
        .map(print_filter)
        .collect::<Vec<_>>()
        .join(separator);
    if nested {
        format!("({joined})")
    } else {
        joined
    }
}

fn print_filter(filter: &Filter) -> String {
    // A filter with neither side prints as nothing, and an empty nested group
    // prints as nothing too — which is how the reference ends up emitting a
    // dangling `AND` for `[age] equals 1 AND <empty group>`. The parser never
    // builds either shape; a hand-built AST can, and reproducing the oddity
    // keeps the two sides' cache keys in step.
    let mut result = if let Some(condition) = &filter.condition {
        print_condition(condition)
    } else if let Some(group) = &filter.group {
        print_group(group, true)
    } else {
        String::new()
    };
    if filter.is_negated() {
        // Parenthesize unless the text is already a group, so `NOT` always
        // takes a bracketed operand — an empty operand included, as `NOT ()`.
        if !result.starts_with('(') {
            result = format!("({result})");
        }
        result = format!("NOT {result}");
    }
    result
}

fn print_condition(condition: &Condition) -> String {
    let column = print_column(&condition.column);
    match condition.function {
        FilterFunction::IsBlank => return format!("{column} IS BLANK"),
        FilterFunction::IsNotBlank => return format!("{column} IS NOT BLANK"),
        _ => {}
    }
    let operator = print_operator(condition.function);
    // Only the first argument is printed; the AST never carries more.
    match condition.args.first() {
        Some(arg) => format!("{column} {operator} {}", print_value(arg)),
        None => format!("{column} {operator}"),
    }
}

/// Bracket the column name, unless it already opens with a bracket — a name
/// stored as `[age]` would otherwise become `[[age]]`.
fn print_column(name: &str) -> String {
    if name.starts_with('[') {
        name.to_string()
    } else {
        format!("[{name}]")
    }
}

fn print_operator(function: FilterFunction) -> &'static str {
    match function {
        FilterFunction::Equals => "equals",
        FilterFunction::GreaterThan => ">",
        FilterFunction::LessThan => "<",
        FilterFunction::GreaterThanOrEqual => ">=",
        FilterFunction::LessThanOrEqual => "<=",
        FilterFunction::Contains => "contains",
        FilterFunction::StartsWith => "STARTS WITH",
        FilterFunction::EndsWith => "ENDS WITH",
        FilterFunction::IsBlank => "IS BLANK",
        FilterFunction::IsNotBlank => "IS NOT BLANK",
    }
}

fn print_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => {
            // The backslash is escaped first, then the quote, so a literal
            // backslash does not swallow the quote's escape.
            let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
            format!("\"{escaped}\"")
        }
        serde_json::Value::Bool(true) => "true".to_string(),
        serde_json::Value::Bool(false) => "false".to_string(),
        serde_json::Value::Number(number) => match number.as_f64() {
            Some(value) => js_number_to_string(value),
            // Unreachable for finite JSON numbers, but a large integer literal
            // is better printed than dropped.
            None => number.to_string(),
        },
        serde_json::Value::Null => "null".to_string(),
        // The reference falls through to `String(value)` here.
        other => js_string_of(other),
    }
}

/// Format a number the way JavaScript's `Number.prototype.toString` does, so
/// that a key built here matches one built in the browser.
///
/// Rust's own `Display` agrees for everyday values but never switches to
/// exponential notation, where JavaScript does so above 10^21 and below 10^-6:
/// `1e21` and `1e-7` print as `1e+21` and `1e-7`, not as long digit strings.
fn js_number_to_string(value: f64) -> String {
    if value == 0.0 {
        // Covers -0.0, which JavaScript prints as `0`.
        return "0".to_string();
    }
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if value < 0.0 {
        return format!("-{}", js_number_to_string(-value));
    }

    // `{:e}` gives the shortest round-tripping digits plus a decimal exponent,
    // which is exactly the `s` and `n` of the spec: `s` is the digit string and
    // `n` is one past the exponent.
    let exp_form = format!("{value:e}");
    let (mantissa, exponent) = exp_form.split_once('e').expect("{:e} always emits an e");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exponent
        .parse::<i32>()
        .expect("{:e} emits a decimal exponent")
        + 1;

    if k <= n && n <= 21 {
        // Whole number: the digits then n - k zeros.
        let mut out = digits;
        out.extend(std::iter::repeat_n('0', (n - k) as usize));
        out
    } else if 0 < n && n <= 21 {
        // A decimal point inside the digits.
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        // A leading zero, then -n zeros, then the digits.
        let zeros = "0".repeat((-n) as usize);
        format!("0.{zeros}{digits}")
    } else {
        // Exponential, with an explicit `+` on a positive exponent.
        let sign = if n >= 1 { "+" } else { "-" };
        let magnitude = (n - 1).abs();
        if k == 1 {
            format!("{digits}e{sign}{magnitude}")
        } else {
            format!("{}.{}e{sign}{magnitude}", &digits[..1], &digits[1..])
        }
    }
}

/// JavaScript's `String(value)` for the JSON values `printValue` does not
/// special-case: an array joins its elements with commas, treating null and
/// undefined as empty, and any other object is `[object Object]`.
fn js_string_of(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Array(items) => items
            .iter()
            .map(|item| match item {
                serde_json::Value::Null => String::new(),
                serde_json::Value::String(text) => text.clone(),
                serde_json::Value::Bool(true) => "true".to_string(),
                serde_json::Value::Bool(false) => "false".to_string(),
                serde_json::Value::Number(number) => match number.as_f64() {
                    Some(value) => js_number_to_string(value),
                    None => number.to_string(),
                },
                nested => js_string_of(nested),
            })
            .collect::<Vec<_>>()
            .join(","),
        serde_json::Value::Object(_) => "[object Object]".to_string(),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Bool(true) => "true".to_string(),
        serde_json::Value::Bool(false) => "false".to_string(),
        serde_json::Value::Number(number) => match number.as_f64() {
            Some(value) => js_number_to_string(value),
            None => number.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_query_unchecked;
    use serde_json::json;

    fn printed(query: &str) -> String {
        to_query_string(&parse_query_unchecked(query).expect(query))
    }

    #[test]
    fn the_measured_round_trips_are_reproduced() {
        assert_eq!(printed("[age]>100"), "[age] > 100");
        assert_eq!(printed("[age] greater than 100"), "[age] > 100");
        assert_eq!(
            printed("[name] not contains \"a\""),
            "NOT ([name] contains \"a\")"
        );
        assert_eq!(printed("NOT [age] > 1"), "NOT ([age] > 1)");
        assert_eq!(
            printed("[age] > 1 AND [name] = \"a\" OR [active] = true"),
            "([age] > 1 AND [name] equals \"a\") OR [active] equals true"
        );
        assert_eq!(printed("[name] is not blank"), "[name] IS NOT BLANK");
    }

    #[test]
    fn every_operator_has_a_canonical_spelling() {
        let group = FilterGroup::and(
            [
                FilterFunction::GreaterThan,
                FilterFunction::LessThan,
                FilterFunction::GreaterThanOrEqual,
                FilterFunction::LessThanOrEqual,
                FilterFunction::Equals,
                FilterFunction::Contains,
                FilterFunction::StartsWith,
                FilterFunction::EndsWith,
            ]
            .into_iter()
            .map(|f| Filter::condition("c", f, vec![json!("v")]))
            .collect(),
        );
        assert_eq!(
            to_query_string(&group),
            "[c] > \"v\" AND [c] < \"v\" AND [c] >= \"v\" AND [c] <= \"v\" \
             AND [c] equals \"v\" AND [c] contains \"v\" \
             AND [c] STARTS WITH \"v\" AND [c] ENDS WITH \"v\""
        );
    }

    #[test]
    fn blank_operators_print_as_keywords() {
        let group = FilterGroup::and(vec![
            Filter::condition("c", FilterFunction::IsBlank, vec![]),
            Filter::condition("c", FilterFunction::IsNotBlank, vec![]),
        ]);
        assert_eq!(to_query_string(&group), "[c] IS BLANK AND [c] IS NOT BLANK");
        // Stray args on a blank operator are ignored, as in the reference.
        let group = FilterGroup::and(vec![Filter::condition(
            "c",
            FilterFunction::IsBlank,
            vec![json!("x")],
        )]);
        assert_eq!(to_query_string(&group), "[c] IS BLANK");
    }

    #[test]
    fn an_empty_group_prints_as_nothing() {
        assert_eq!(to_query_string(&FilterGroup::default()), "");
        assert_eq!(to_query_string(&FilterGroup::or(vec![])), "");
        assert_eq!(printed(""), "");
        assert_eq!(printed("   "), "");
    }

    #[test]
    fn a_false_negate_key_prints_nothing_extra() {
        let group = FilterGroup::and(vec![Filter::condition(
            "name",
            FilterFunction::Contains,
            vec![json!("a")],
        )
        .negated(false)]);
        assert_eq!(to_query_string(&group), "[name] contains \"a\"");
        // Which is exactly what the parser produces for the un-negated form.
        assert_eq!(printed("[name] contains \"a\""), "[name] contains \"a\"");
    }

    #[test]
    fn nested_groups_are_parenthesized() {
        assert_eq!(
            printed("([age] > 1 AND [b] = 2) OR [c] = 3"),
            "([age] > 1 AND [b] equals 2) OR [c] equals 3"
        );
        assert_eq!(
            printed("(([age] > 1))"),
            "(([age] > 1))",
            "parens are not collapsed"
        );
        assert_eq!(
            printed("NOT ([age] > 1 AND [b] = 2)"),
            "NOT ([age] > 1 AND [b] equals 2)"
        );
    }

    #[test]
    fn negation_of_an_empty_group_still_brackets() {
        let group = FilterGroup::and(vec![
            Filter::from_group(FilterGroup::default()).negated(true)
        ]);
        assert_eq!(to_query_string(&group), "NOT ()");
        let group = FilterGroup::and(vec![Filter::default().negated(true)]);
        assert_eq!(to_query_string(&group), "NOT ()");
    }

    #[test]
    fn a_filter_with_neither_side_prints_as_nothing() {
        let group = FilterGroup::and(vec![Filter::default()]);
        assert_eq!(to_query_string(&group), "");
        // And an empty arm leaves a dangling operator, faithfully.
        let group = FilterGroup::and(vec![
            Filter::condition("age", FilterFunction::Equals, vec![json!(1)]),
            Filter::from_group(FilterGroup::default()),
        ]);
        assert_eq!(to_query_string(&group), "[age] equals 1 AND ");
    }

    #[test]
    fn a_condition_with_no_args_prints_the_bare_operator() {
        let group = FilterGroup::and(vec![Filter::condition(
            "age",
            FilterFunction::GreaterThan,
            vec![],
        )]);
        assert_eq!(to_query_string(&group), "[age] >");
    }

    #[test]
    fn columns_are_bracketed_at_most_once() {
        let group = FilterGroup::and(vec![
            Filter::condition("age", FilterFunction::Equals, vec![json!(1)]),
            Filter::condition("[age]", FilterFunction::Equals, vec![json!(1)]),
            Filter::condition("a]b", FilterFunction::Equals, vec![json!(1)]),
        ]);
        assert_eq!(
            to_query_string(&group),
            "[age] equals 1 AND [age] equals 1 AND [a]b] equals 1"
        );
    }

    #[test]
    fn strings_escape_the_backslash_before_the_quote() {
        let cases = [
            ("a", "\"a\""),
            ("", "\"\""),
            ("\"", "\"\\\"\""),
            ("\\", "\"\\\\\""),
            ("\\\"", "\"\\\\\\\"\""),
            ("\\\\", "\"\\\\\\\\\""),
            // Whitespace and single quotes pass through untouched.
            ("\t", "\"\t\""),
            ("\n", "\"\n\""),
            ("'", "\"'\""),
        ];
        for (input, expected) in cases {
            let group = FilterGroup::and(vec![Filter::condition(
                "c",
                FilterFunction::Equals,
                vec![json!(input)],
            )]);
            assert_eq!(
                to_query_string(&group),
                format!("[c] equals {expected}"),
                "{input:?}"
            );
        }
    }

    #[test]
    fn a_quoted_string_survives_the_round_trip() {
        let query = "[c] = \"a\\\"b\\\\c\"";
        let group = parse_query_unchecked(query).unwrap();
        let printed = to_query_string(&group);
        assert_eq!(printed, "[c] equals \"a\\\"b\\\\c\"");
        assert_eq!(parse_query_unchecked(&printed).unwrap(), group);
    }

    #[test]
    fn booleans_and_null_print_as_literals() {
        let group = FilterGroup::and(vec![
            Filter::condition("b", FilterFunction::Equals, vec![json!(true)]),
            Filter::condition("b", FilterFunction::Equals, vec![json!(false)]),
            Filter::condition("b", FilterFunction::Equals, vec![serde_json::Value::Null]),
        ]);
        assert_eq!(
            to_query_string(&group),
            "[b] equals true AND [b] equals false AND [b] equals null"
        );
    }

    #[test]
    fn numbers_print_the_way_javascript_prints_them() {
        // Every expectation measured against `String(n)` in Node.
        let cases = [
            (1.0, "1"),
            (-1.0, "-1"),
            (0.0, "0"),
            (-0.0, "0"),
            (1.5, "1.5"),
            (0.1, "0.1"),
            (100.0, "100"),
            (1e21, "1e+21"),
            (1e-7, "1e-7"),
            (-1e-7, "-1e-7"),
            (1e20, "100000000000000000000"),
            (1e-6, "0.000001"),
            (1.2345e2, "123.45"),
            (1.5e22, "1.5e+22"),
            (1.25e-8, "1.25e-8"),
            (f64::INFINITY, "Infinity"),
            (f64::NEG_INFINITY, "-Infinity"),
            (f64::NAN, "NaN"),
        ];
        for (value, expected) in cases {
            assert_eq!(js_number_to_string(value), expected, "{value}");
        }
    }

    #[test]
    fn numbers_from_the_parser_print_back_identically() {
        for query in [
            "[n] = 0",
            "[n] = 1",
            "[n] = -1",
            "[n] = 1.5",
            "[n] = 0.1",
            "[n] = 100",
            "[n] = 100000000000000000000",
        ] {
            assert_eq!(printed(query), query.replace('=', "equals"), "{query}");
        }
        // A literal too large for an f64 becomes `Infinity`, which the
        // reference stores as null; printing says `null` rather than inventing
        // a value.
        let huge = format!("[n] = 1{}", "0".repeat(400));
        assert_eq!(printed(&huge), "[n] equals null");
    }

    #[test]
    fn composite_args_fall_back_to_the_javascript_string_conversion() {
        let group = FilterGroup::and(vec![Filter::condition(
            "c",
            FilterFunction::Equals,
            vec![json!([1, 2])],
        )]);
        assert_eq!(to_query_string(&group), "[c] equals 1,2");
        let group = FilterGroup::and(vec![Filter::condition(
            "c",
            FilterFunction::Equals,
            vec![json!({"a": 1})],
        )]);
        assert_eq!(to_query_string(&group), "[c] equals [object Object]");
        // Null inside an array is the empty string, not `null`.
        let group = FilterGroup::and(vec![Filter::condition(
            "c",
            FilterFunction::Equals,
            vec![json!([null, "a", [2, 3]])],
        )]);
        assert_eq!(to_query_string(&group), "[c] equals ,a,2,3");
    }

    #[test]
    fn only_the_first_arg_is_printed() {
        let group = FilterGroup::and(vec![Filter::condition(
            "c",
            FilterFunction::Equals,
            vec![json!("a"), json!("b")],
        )]);
        assert_eq!(to_query_string(&group), "[c] equals \"a\"");
    }

    #[test]
    fn canonical_key_is_the_query_string() {
        let group = parse_query_unchecked("[age]>100").unwrap();
        assert_eq!(canonical_key(&group), to_query_string(&group));
        assert_eq!(canonical_key(&FilterGroup::default()), "");
    }

    #[test]
    fn synonyms_collapse_onto_one_key() {
        // The point of a canonical key: differently spelled equivalents agree.
        let keys = ["[age]>100", "[age] > 100", "[age] greater than 100"]
            .map(|q| canonical_key(&parse_query_unchecked(q).unwrap()));
        assert_eq!(keys[0], keys[1]);
        assert_eq!(keys[1], keys[2]);
    }
}
