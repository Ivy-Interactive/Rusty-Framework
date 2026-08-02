//! The filter AST, mirroring `dist/types/filter.d.ts` of `filter-query-editor`.
//!
//! The shapes here are deliberately faithful to the TypeScript interfaces,
//! `Option` placement included, so that `serde_json` output is byte-compatible
//! with what the browser's editor produces. See the crate-level docs for the
//! rules governing which keys are present.

use serde::{Deserialize, Serialize};

/// How the filters of a group are combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LogicalOp {
    /// Every filter must match. This is the default, matching the empty-query
    /// result `{op: "AND", filters: []}`.
    #[default]
    #[serde(rename = "AND")]
    And,
    #[serde(rename = "OR")]
    Or,
}

/// The ten functions the reference implementation emits.
///
/// There is deliberately no `NotEquals`: `!=`, `not equals` and `not equal` all
/// produce [`FilterFunction::Equals`] with `negate: Some(true)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FilterFunction {
    Equals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    StartsWith,
    EndsWith,
    IsBlank,
    IsNotBlank,
}

impl FilterFunction {
    /// The human-readable name used in validation messages, from
    /// `getOperatorDisplayName` in `dist/validator/TypeChecker.js`.
    pub fn display_name(self) -> &'static str {
        match self {
            FilterFunction::Equals => "equals",
            FilterFunction::GreaterThan => "greater than",
            FilterFunction::LessThan => "less than",
            FilterFunction::GreaterThanOrEqual => "greater than or equal",
            FilterFunction::LessThanOrEqual => "less than or equal",
            FilterFunction::Contains => "contains",
            FilterFunction::StartsWith => "starts with",
            FilterFunction::EndsWith => "ends with",
            FilterFunction::IsBlank => "is blank",
            FilterFunction::IsNotBlank => "is not blank",
        }
    }

    /// Whether this is one of the two argument-less existence operators.
    pub fn is_blank_operator(self) -> bool {
        matches!(self, FilterFunction::IsBlank | FilterFunction::IsNotBlank)
    }
}

/// A single filter expression, e.g. `[status] equals "open"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Condition {
    /// The column identifier, exactly as written between the brackets.
    pub column: String,
    /// The comparison or text operation.
    pub function: FilterFunction,
    /// Values to compare against. Empty for the two blank operators.
    pub args: Vec<serde_json::Value>,
}

impl Condition {
    pub fn new(
        column: impl Into<String>,
        function: FilterFunction,
        args: Vec<serde_json::Value>,
    ) -> Self {
        Condition {
            column: column.into(),
            function,
            args,
        }
    }
}

/// A collection of filters combined with `AND` or `OR`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FilterGroup {
    pub op: LogicalOp,
    pub filters: Vec<Filter>,
}

impl FilterGroup {
    /// A group combining `filters` with `AND`.
    pub fn and(filters: Vec<Filter>) -> Self {
        FilterGroup {
            op: LogicalOp::And,
            filters,
        }
    }

    /// A group combining `filters` with `OR`.
    pub fn or(filters: Vec<Filter>) -> Self {
        FilterGroup {
            op: LogicalOp::Or,
            filters,
        }
    }

    /// Whether this group holds no filters. An empty group is what the empty
    /// query parses to, and it is the case [`crate::print::canonical_key`]
    /// treats as "no filter at all".
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }
}

/// One entry of a [`FilterGroup`]: a condition or a nested group, optionally
/// negated.
///
/// `condition` and `group` are mutually exclusive but both optional, as in the
/// TypeScript interface. `negate` is `Option<bool>` on purpose: comparisons omit
/// the key entirely, text operations always emit it (`false` included).
/// Round-tripping the frontend's JSON must not change which keys are present.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Filter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<Condition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<FilterGroup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negate: Option<bool>,
}

impl Filter {
    /// A condition filter with no `negate` key, as a comparison produces.
    pub fn condition(
        column: impl Into<String>,
        function: FilterFunction,
        args: Vec<serde_json::Value>,
    ) -> Self {
        Filter {
            condition: Some(Condition::new(column, function, args)),
            group: None,
            negate: None,
        }
    }

    /// A filter wrapping a nested group, as `(...)` produces.
    pub fn from_group(group: FilterGroup) -> Self {
        Filter {
            condition: None,
            group: Some(group),
            negate: None,
        }
    }

    /// This filter with `negate: Some(negate)`. Passing `false` emits the key
    /// with a `false` value rather than dropping it, which is what a text
    /// operation without `NOT` does.
    pub fn negated(mut self, negate: bool) -> Self {
        self.negate = Some(negate);
        self
    }

    /// Whether negation is in effect. A missing key and `Some(false)` both mean
    /// "not negated", exactly as the JavaScript truthiness test does.
    pub fn is_negated(&self) -> bool {
        self.negate.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn logical_op_defaults_to_and() {
        assert_eq!(LogicalOp::default(), LogicalOp::And);
        assert_eq!(FilterGroup::default(), FilterGroup::and(vec![]));
    }

    #[test]
    fn logical_op_serializes_uppercase() {
        assert_eq!(serde_json::to_value(LogicalOp::And).unwrap(), json!("AND"));
        assert_eq!(serde_json::to_value(LogicalOp::Or).unwrap(), json!("OR"));
    }

    #[test]
    fn filter_function_serializes_camel_case() {
        let pairs = [
            (FilterFunction::Equals, "equals"),
            (FilterFunction::GreaterThan, "greaterThan"),
            (FilterFunction::LessThan, "lessThan"),
            (FilterFunction::GreaterThanOrEqual, "greaterThanOrEqual"),
            (FilterFunction::LessThanOrEqual, "lessThanOrEqual"),
            (FilterFunction::Contains, "contains"),
            (FilterFunction::StartsWith, "startsWith"),
            (FilterFunction::EndsWith, "endsWith"),
            (FilterFunction::IsBlank, "isBlank"),
            (FilterFunction::IsNotBlank, "isNotBlank"),
        ];
        for (func, expected) in pairs {
            assert_eq!(serde_json::to_value(func).unwrap(), json!(expected));
        }
    }

    #[test]
    fn display_names_match_the_reference() {
        assert_eq!(FilterFunction::Equals.display_name(), "equals");
        assert_eq!(FilterFunction::GreaterThan.display_name(), "greater than");
        assert_eq!(FilterFunction::LessThan.display_name(), "less than");
        assert_eq!(
            FilterFunction::GreaterThanOrEqual.display_name(),
            "greater than or equal"
        );
        assert_eq!(
            FilterFunction::LessThanOrEqual.display_name(),
            "less than or equal"
        );
        assert_eq!(FilterFunction::Contains.display_name(), "contains");
        assert_eq!(FilterFunction::StartsWith.display_name(), "starts with");
        assert_eq!(FilterFunction::EndsWith.display_name(), "ends with");
        assert_eq!(FilterFunction::IsBlank.display_name(), "is blank");
        assert_eq!(FilterFunction::IsNotBlank.display_name(), "is not blank");
    }

    #[test]
    fn absent_negate_is_omitted_from_json() {
        let f = Filter::condition("age", FilterFunction::GreaterThan, vec![json!(1)]);
        let value = serde_json::to_value(&f).unwrap();
        assert_eq!(
            value,
            json!({"condition": {"column": "age", "function": "greaterThan", "args": [1]}})
        );
        assert!(value.get("negate").is_none());
        assert!(value.get("group").is_none());
    }

    #[test]
    fn false_negate_is_present_in_json() {
        let f =
            Filter::condition("name", FilterFunction::Contains, vec![json!("a")]).negated(false);
        let value = serde_json::to_value(&f).unwrap();
        assert_eq!(value.get("negate"), Some(&json!(false)));
    }

    #[test]
    fn is_negated_treats_missing_and_false_alike() {
        let bare = Filter::condition("age", FilterFunction::Equals, vec![json!(1)]);
        assert!(!bare.is_negated());
        assert!(!bare.clone().negated(false).is_negated());
        assert!(bare.negated(true).is_negated());
    }

    #[test]
    fn group_constructors_set_the_operator() {
        assert_eq!(FilterGroup::and(vec![]).op, LogicalOp::And);
        assert_eq!(FilterGroup::or(vec![]).op, LogicalOp::Or);
        assert!(FilterGroup::and(vec![]).is_empty());
        assert!(!FilterGroup::and(vec![Filter::default()]).is_empty());
    }

    #[test]
    fn from_group_wraps_without_negation() {
        let inner = FilterGroup::or(vec![Filter::condition(
            "age",
            FilterFunction::Equals,
            vec![json!(1)],
        )]);
        let f = Filter::from_group(inner.clone());
        assert_eq!(f.group, Some(inner));
        assert!(f.condition.is_none());
        assert!(f.negate.is_none());
    }

    #[test]
    fn blank_operators_are_recognised() {
        assert!(FilterFunction::IsBlank.is_blank_operator());
        assert!(FilterFunction::IsNotBlank.is_blank_operator());
        assert!(!FilterFunction::Equals.is_blank_operator());
        assert!(!FilterFunction::Contains.is_blank_operator());
    }

    #[test]
    fn round_trips_through_json() {
        let group = FilterGroup::or(vec![
            Filter::from_group(FilterGroup::and(vec![
                Filter::condition("age", FilterFunction::GreaterThan, vec![json!(1)]),
                Filter::condition("name", FilterFunction::Contains, vec![json!("a")])
                    .negated(false),
            ])),
            Filter::condition("active", FilterFunction::Equals, vec![json!(true)]).negated(true),
        ]);
        let json = serde_json::to_string(&group).unwrap();
        let back: FilterGroup = serde_json::from_str(&json).unwrap();
        assert_eq!(back, group);
    }
}
