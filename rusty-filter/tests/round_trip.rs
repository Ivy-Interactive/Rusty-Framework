//! Printing an AST and parsing the result back.
//!
//! This is what makes [`canonical_key`] usable as a cache key: two subscribers
//! who wrote the same filter differently must land on one string, and that string
//! must still mean what they wrote.
//!
//! Assertions here are at the **AST** level, never on string equality.
//! `canonical_key` is not string-idempotent on the first pass — `[name] not
//! contains "a"` prints as `NOT ([name] contains "a")`, which is a different
//! source text — so a string round trip would fail on correct output.
//!
//! # Negated leaves are not AST-stable, in either implementation
//!
//! AST stability does not hold universally, and the reference does not have it
//! either. A negated *leaf* prints as `NOT (...)`, and re-parsing that reads the
//! parentheses as a group, so the negation moves from the condition filter onto a
//! wrapping group filter:
//!
//! ```text
//! [age] != 100
//!   -> {op:AND, filters:[{condition:{...equals...}, negate:true}]}
//!   -> prints as "NOT ([age] equals 100)"
//!   -> {op:AND, filters:[{group:{op:AND, filters:[{condition:{...}}]}, negate:true}]}
//! ```
//!
//! Measured against `filter-query-editor` 2.2.0 over the 47 queries in
//! [`QUERIES`]: 39 are AST-stable and the 8 in [`NOT_AST_STABLE`] are not, with
//! the bundle producing the very same reshaped AST this crate does. So the
//! divergence is shared, not introduced here — and the tests below pin both
//! halves: exact AST equality for the stable set, and semantic equivalence plus
//! string idempotence for the rest.

use rusty_filter::{
    canonical_key, evaluate, parse_query, parse_query_unchecked, to_query_string, ColumnDef,
    ColumnType, FilterGroup,
};

fn columns() -> Vec<ColumnDef> {
    vec![
        ColumnDef::new("age", ColumnType::Number),
        ColumnDef::new("name", ColumnType::String),
        ColumnDef::new("active", ColumnType::Boolean),
        ColumnDef::new("when", ColumnType::Date),
        ColumnDef::new("kind", ColumnType::Enum),
    ]
}

/// Rows covering the truth table of every operator the queries below use.
fn rows() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({"age": 100, "name": "ab", "active": true, "when": "2024-01-01", "kind": "a"}),
        serde_json::json!({"age": 101, "name": "abc", "active": false, "when": "2025-06-30T00:00:00Z", "kind": ""}),
        serde_json::json!({"age": 1, "name": "zab", "active": true, "when": "2023-01-01", "kind": "b"}),
        serde_json::json!({"age": -5, "name": "", "active": false, "when": null, "kind": null}),
        serde_json::json!({"age": 1.5, "name": "a\"b", "active": true}),
        serde_json::json!({"age": 7, "name": "a\\b", "active": false, "when": "2024-01-01"}),
        serde_json::json!({}),
    ]
}

/// Every valid query shape the grammar admits, one per feature.
const QUERIES: &[&str] = &[
    // comparisons, symbolic and spelled out
    "[age] > 100",
    "[age] >= 100",
    "[age] < 100",
    "[age] <= 100",
    "[age] = 100",
    "[age] == 100",
    "[age] != 100",
    "[age] equals 100",
    "[age] not equals 100",
    "[age] not equal 100",
    "[age] greater than 100",
    "[age] greater than or equal 100",
    "[age] less than 100",
    "[age] less than or equal 100",
    // text operations, negated and not
    r#"[name] contains "ab""#,
    r#"[name] not contains "ab""#,
    r#"[name] starts with "ab""#,
    r#"[name] not starts with "ab""#,
    r#"[name] ends with "ab""#,
    r#"[name] not ends with "ab""#,
    // existence
    "[name] is blank",
    "[name] is not blank",
    "[when] is blank",
    "[kind] is not blank",
    // literals
    "[active] = true",
    "[active] = false",
    "[age] = -5",
    "[age] = 1.5",
    "[age] = 007",
    r#"[when] = "2024-01-01""#,
    r#"[when] > "2024-01-01T10:20:30.123Z""#,
    // strings needing escapes
    r#"[name] = "a\"b""#,
    r#"[name] = "a\\b""#,
    // logical structure
    r#"[age] > 1 AND [name] = "a""#,
    r#"[age] > 1 OR [name] = "a""#,
    r#"[age] > 1 AND [name] = "a" AND [active] = true"#,
    r#"[age] > 1 OR [name] = "a" OR [active] = true"#,
    r#"[age] > 1 AND [name] = "a" OR [active] = true"#,
    r#"[age] > 1 OR [name] = "a" AND [active] = true"#,
    // negation and parentheses
    "NOT [age] > 1",
    "not not [age] > 1",
    "(([age] > 1))",
    r#"NOT ([age] > 1 AND [name] = "a")"#,
    r#"([age] > 1 AND [name] = "a") OR [active] = true"#,
    r#"([age] > 1 OR [name] = "a") AND [active] = true"#,
    // empty
    "",
    "   ",
];

/// The queries whose AST is reshaped by a print/parse round trip, because their
/// negation sits on a leaf and printing lifts it onto a `NOT (...)` group.
///
/// This list is the bundle's, not this crate's: each entry was confirmed to be
/// AST-unstable in `filter-query-editor` 2.2.0 as well. Anything not listed here
/// must be exactly stable, which [`printing_then_reparsing_preserves_the_ast`]
/// enforces — so an implementation change that quietly reshapes one more query
/// fails the suite rather than growing this list.
const NOT_AST_STABLE: &[&str] = &[
    "[age] != 100",
    "[age] not equals 100",
    "[age] not equal 100",
    r#"[name] not contains "ab""#,
    r#"[name] not starts with "ab""#,
    r#"[name] not ends with "ab""#,
    "NOT [age] > 1",
    "not not [age] > 1",
];

fn parse(query: &str) -> FilterGroup {
    parse_query(query, &columns())
        .filters
        .unwrap_or_else(|| panic!("{query} should parse"))
}

#[test]
fn printing_then_reparsing_preserves_the_ast() {
    let mut stable = 0;
    for query in QUERIES {
        let first = parse(query);
        let printed = to_query_string(&first);
        let second = parse(&printed);
        if NOT_AST_STABLE.contains(query) {
            assert_ne!(
                second, first,
                "{query} is listed as AST-unstable but round-tripped cleanly — \
                 drop it from NOT_AST_STABLE"
            );
        } else {
            assert_eq!(second, first, "{query} printed as {printed:?}");
            stable += 1;
        }
    }
    // Pins the split measured against the bundle: 39 of 47 stable.
    assert_eq!(stable, QUERIES.len() - NOT_AST_STABLE.len());
    assert_eq!(stable, 39);
}

#[test]
fn a_reshaped_round_trip_still_means_the_same_thing() {
    // The negated-leaf reshaping moves `negate` onto a wrapping group, which is a
    // different AST but the same predicate. That is the property that actually
    // matters for a cache key, so it is asserted for *every* query rather than
    // only the unstable ones.
    for query in QUERIES {
        let first = parse(query);
        let second = parse(&to_query_string(&first));
        let cols = columns();
        for row in rows() {
            assert_eq!(
                evaluate(&first, &row, &cols),
                evaluate(&second, &row, &cols),
                "{query} disagreed on row {row}"
            );
        }
    }
}

#[test]
fn a_second_round_trip_is_a_fixed_point() {
    // Printing is idempotent from the second pass onwards even where the AST is
    // reshaped on the first, so a cache key derived from a cache key is the same
    // key. Without this, `canonical_key` would not be canonical.
    for query in QUERIES {
        let once = to_query_string(&parse(query));
        let twice = to_query_string(&parse(&once));
        assert_eq!(twice, once, "{query}");
        let thrice = to_query_string(&parse(&twice));
        assert_eq!(thrice, once, "{query}");
    }
}

#[test]
fn equivalent_spellings_share_one_cache_key() {
    for [a, b] in [
        ["[age] > 100", "[age] greater than 100"],
        ["[age] >= 100", "[age] greater than or equal 100"],
        ["[age] < 100", "[age] less than 100"],
        ["[age] <= 100", "[age] less than or equal 100"],
        ["[age] = 100", "[age] == 100"],
        ["[age] != 100", "[age] not equals 100"],
        ["[age] != 100", "[age] not equal 100"],
        ["[age] = 7", "[age] = 007"],
        ["[age] = 1.5", "[age] = 1 . 5"],
        ["[age] > 1", "not not [age] > 1"],
        // Keyword case is folded, so only the keywords differ here.
        ["[age] > 1", "[age] GREATER THAN 1"],
        [r#"[name] contains "a""#, r#"[name] CONTAINS "a""#],
    ] {
        let key_a = canonical_key(&parse_query_unchecked(a).expect("valid"));
        let key_b = canonical_key(&parse_query_unchecked(b).expect("valid"));
        assert_eq!(key_a, key_b, "{a:?} vs {b:?}");
    }
}

#[test]
fn a_column_name_keeps_its_case_in_the_key() {
    // Keyword case is folded but a column name is not, so two differently-cased
    // column names are two different cache keys — as they must be, since the
    // column lookup at evaluation time is case-sensitive too.
    let lower = canonical_key(&parse_query_unchecked("[name] = \"a\"").expect("valid"));
    let upper = canonical_key(&parse_query_unchecked("[NAME] = \"a\"").expect("valid"));
    assert_ne!(lower, upper);
}

#[test]
fn differently_meaning_queries_get_different_keys() {
    // The other half of a cache key's contract: collapsing two *different*
    // filters onto one key would serve one subscriber the other's rows.
    let queries = [
        "[age] > 1",
        "[age] >= 1",
        "[age] > 2",
        "[age] < 1",
        "NOT [age] > 1",
        r#"[age] > 1 AND [name] = "a""#,
        r#"[age] > 1 OR [name] = "a""#,
        r#"[name] contains "a""#,
        r#"[name] not contains "a""#,
        r#"[name] starts with "a""#,
        r#"[name] ends with "a""#,
        "[name] is blank",
        "[name] is not blank",
        "",
    ];
    let keys: Vec<String> = queries
        .iter()
        .map(|q| canonical_key(&parse_query_unchecked(q).expect("valid")))
        .collect();
    let mut unique = keys.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), keys.len(), "keys collided: {keys:?}");
}
