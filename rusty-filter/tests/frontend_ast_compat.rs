//! The AST this crate serializes must be byte-compatible with the JSON the
//! browser's `filter-query-editor` produces, because both halves of a filter can
//! be authored on either side and a `DataTable` query has to mean one thing.
//!
//! Every expected value below is the **verbatim output of the shipped bundle**,
//! captured by importing `dist/index.js` from
//! `src/frontend/node_modules/filter-query-editor` (version 2.2.0) and calling
//! `parseQuery(query, columns)` with the column schema in [`columns`]. Nothing
//! here is hand-written from the grammar: the point of this file is to catch
//! drift away from the reference, and a hand-written expectation would drift with
//! the implementation instead of pinning it.
//!
//! Reproducing the capture:
//!
//! ```js
//! const B = '<repo>/src/frontend/node_modules/filter-query-editor/dist/index.js';
//! const { parseQuery } = await import(B);
//! console.log(JSON.stringify(parseQuery('[age] > 1', cols)));
//! ```
//!
//! The one thing that is *not* asserted verbatim is a multi-error cascade: on a
//! syntax error this crate reports the first error and stops, which the crate
//! docs call out as a deliberate divergence. Single-error inputs are compared in
//! full, spans included.

use rusty_filter::{parse_query, ColumnDef, ColumnType};

/// The schema the probe ran with. `n` and `x` exist only to keep the captured
/// mixed `AND`/`OR` query short.
fn columns() -> Vec<ColumnDef> {
    vec![
        ColumnDef::new("age", ColumnType::Number),
        ColumnDef::new("name", ColumnType::String),
        ColumnDef::new("active", ColumnType::Boolean),
        ColumnDef::new("when", ColumnType::Date),
        ColumnDef::new("kind", ColumnType::Enum),
        ColumnDef::new("n", ColumnType::String),
        ColumnDef::new("x", ColumnType::Boolean),
    ]
}

/// Assert that this crate's `ParseResult` serializes to exactly `expected`.
///
/// Comparing `serde_json::Value` rather than strings keeps key *presence*
/// significant while leaving key order free, which is what matters: the
/// reference omits `negate` on comparisons and emits `negate: false` on text
/// operations, and a round trip through the browser must not change that.
#[track_caller]
fn assert_bundle_json(query: &str, expected: &str) {
    let expected: serde_json::Value =
        serde_json::from_str(expected).expect("the captured JSON parses");
    let actual = serde_json::to_value(parse_query(query, &columns())).expect("the AST serializes");
    assert_eq!(actual, expected, "query: {query}");
}

#[test]
fn a_comparison_omits_the_negate_key() {
    assert_bundle_json(
        "[age] > 1",
        r#"{"filters":{"op":"AND","filters":[{"condition":{"column":"age","function":"greaterThan","args":[1]}}]}}"#,
    );
    assert_bundle_json(
        "[age] = 100",
        r#"{"filters":{"op":"AND","filters":[{"condition":{"column":"age","function":"equals","args":[100]}}]}}"#,
    );
}

#[test]
fn a_text_operation_emits_negate_false() {
    assert_bundle_json(
        r#"[name] contains "ab""#,
        r#"{"filters":{"op":"AND","filters":[{"condition":{"column":"name","function":"contains","args":["ab"]},"negate":false}]}}"#,
    );
    assert_bundle_json(
        r#"[name] starts with "a""#,
        r#"{"filters":{"op":"AND","filters":[{"condition":{"column":"name","function":"startsWith","args":["a"]},"negate":false}]}}"#,
    );
}

#[test]
fn negated_forms_carry_negate_true() {
    // `!=` is `equals` with `negate: true` — there is no `notEquals` function.
    assert_bundle_json(
        "[age] != 5",
        r#"{"filters":{"op":"AND","filters":[{"condition":{"column":"age","function":"equals","args":[5]},"negate":true}]}}"#,
    );
    assert_bundle_json(
        r#"[name] not contains "ab""#,
        r#"{"filters":{"op":"AND","filters":[{"condition":{"column":"name","function":"contains","args":["ab"]},"negate":true}]}}"#,
    );
    // `NOT` lands on the condition filter itself, not on a wrapper.
    assert_bundle_json(
        "NOT [age] > 1",
        r#"{"filters":{"op":"AND","filters":[{"condition":{"column":"age","function":"greaterThan","args":[1]},"negate":true}]}}"#,
    );
    // And it *toggles*, so a double `NOT` is `false` rather than absent.
    assert_bundle_json(
        "not not [age] > 1",
        r#"{"filters":{"op":"AND","filters":[{"condition":{"column":"age","function":"greaterThan","args":[1]},"negate":false}]}}"#,
    );
}

#[test]
fn an_existence_operation_has_empty_args_and_no_negate() {
    assert_bundle_json(
        "[name] is blank",
        r#"{"filters":{"op":"AND","filters":[{"condition":{"column":"name","function":"isBlank","args":[]}}]}}"#,
    );
    assert_bundle_json(
        "[name] is not blank",
        r#"{"filters":{"op":"AND","filters":[{"condition":{"column":"name","function":"isNotBlank","args":[]}}]}}"#,
    );
}

#[test]
fn mixed_and_or_nests_the_and_arm_as_a_group() {
    // `OR` is the root, and the `AND` arm is spliced in as `{group: ...}`.
    assert_bundle_json(
        r#"[age] > 1 AND [n] = "a" OR [x] = true"#,
        r#"{"filters":{"op":"OR","filters":[{"group":{"op":"AND","filters":[{"condition":{"column":"age","function":"greaterThan","args":[1]}},{"condition":{"column":"n","function":"equals","args":["a"]}}]}},{"condition":{"column":"x","function":"equals","args":[true]}}]}}"#,
    );
}

#[test]
fn parentheses_are_never_collapsed() {
    assert_bundle_json(
        "(([age] > 1))",
        r#"{"filters":{"op":"AND","filters":[{"group":{"op":"AND","filters":[{"group":{"op":"AND","filters":[{"condition":{"column":"age","function":"greaterThan","args":[1]}}]}}]}}]}}"#,
    );
}

#[test]
fn an_empty_query_is_an_empty_and_group() {
    let empty = r#"{"filters":{"op":"AND","filters":[]}}"#;
    assert_bundle_json("", empty);
    assert_bundle_json("   ", empty);
}

#[test]
fn numbers_serialize_as_javascript_writes_them() {
    // JavaScript has one number type, so `007` is `7` and an integral fraction
    // loses its decimal point. Whitespace inside a number is skipped.
    assert_bundle_json(
        "[age] = 007",
        r#"{"filters":{"op":"AND","filters":[{"condition":{"column":"age","function":"equals","args":[7]}}]}}"#,
    );
    assert_bundle_json(
        "[age] = 1 . 5",
        r#"{"filters":{"op":"AND","filters":[{"condition":{"column":"age","function":"equals","args":[1.5]}}]}}"#,
    );
    assert_bundle_json(
        "[age] = 1.000",
        r#"{"filters":{"op":"AND","filters":[{"condition":{"column":"age","function":"equals","args":[1]}}]}}"#,
    );
}

#[test]
fn a_semantic_error_matches_the_reference_wording_and_zero_span() {
    assert_bundle_json(
        "[nope] = 1",
        r#"{"errors":[{"message":"Column 'nope' does not exist","start":0,"end":0,"severity":"error"}]}"#,
    );
    assert_bundle_json(
        r#"[age] contains "x""#,
        r#"{"errors":[{"message":"Operator 'contains' is not compatible with type 'number'","start":0,"end":0,"severity":"error"}]}"#,
    );
    assert_bundle_json(
        r#"[age] > "x""#,
        r#"{"errors":[{"message":"Expected number for column 'age', got string","start":0,"end":0,"severity":"error"}]}"#,
    );
}

#[test]
fn a_syntax_error_matches_the_reference_message_and_span() {
    // Single-error inputs only: the ANTLR cascade is out of scope, so an input
    // the reference reports three errors for would not compare here.
    assert_bundle_json(
        "[age] > 1 1",
        r#"{"errors":[{"message":"extraneous input '1' expecting <EOF>","start":10,"end":11,"severity":"error"}]}"#,
    );
    assert_bundle_json(
        "[age] = 1e5",
        r#"{"errors":[{"message":"token recognition error at: 'e5'","start":9,"end":10,"severity":"error"}]}"#,
    );
    assert_bundle_json(
        "([age] > 1",
        r#"{"errors":[{"message":"missing ')' at '<EOF>'","start":10,"end":10,"severity":"error"}]}"#,
    );
}
