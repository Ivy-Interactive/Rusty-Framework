//! Column schema used for validation and evaluation.

use serde::{Deserialize, Serialize};

/// The five column types the grammar's validator knows about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColumnType {
    #[default]
    String,
    Number,
    Boolean,
    Date,
    Enum,
}

impl ColumnType {
    /// Map a backend type name onto a [`ColumnType`], reproducing
    /// `normalizeColumnType` in `dist/validator/TypeChecker.js`.
    ///
    /// `INT32`, `INT64`, `DOUBLE`, `DECIMAL` and `NUMBER` become
    /// [`ColumnType::Number`]; `TEXT`, `STRING` and `ICON` become
    /// [`ColumnType::String`]; `BOOLEAN` becomes [`ColumnType::Boolean`];
    /// `DATE` and `DATETIME` become [`ColumnType::Date`]; `ENUM` becomes
    /// [`ColumnType::Enum`]. Anything else becomes [`ColumnType::String`].
    /// The comparison is case-insensitive.
    pub fn normalize(type_name: &str) -> ColumnType {
        let upper = type_name.to_ascii_uppercase();
        match upper.as_str() {
            "INT32" | "INT64" | "DOUBLE" | "DECIMAL" | "NUMBER" => ColumnType::Number,
            "TEXT" | "STRING" | "ICON" => ColumnType::String,
            "BOOLEAN" => ColumnType::Boolean,
            "DATE" | "DATETIME" => ColumnType::Date,
            "ENUM" => ColumnType::Enum,
            _ => ColumnType::String,
        }
    }

    /// The name used in validation messages, matching the lowercase
    /// `ColumnType` union of `dist/types/column.d.ts`.
    pub fn as_str(self) -> &'static str {
        match self {
            ColumnType::String => "string",
            ColumnType::Number => "number",
            ColumnType::Boolean => "boolean",
            ColumnType::Date => "date",
            ColumnType::Enum => "enum",
        }
    }
}

/// One filterable column.
///
/// The TypeScript `ColumnDef` also carries a `width`; nothing in parsing,
/// validation or evaluation reads it, so it is deliberately dropped here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    #[serde(rename = "type")]
    pub column_type: ColumnType,
}

impl ColumnDef {
    pub fn new(name: impl Into<String>, column_type: ColumnType) -> Self {
        ColumnDef {
            name: name.into(),
            column_type,
        }
    }

    /// A column whose backend type name needs normalizing first.
    pub fn normalized(name: impl Into<String>, type_name: &str) -> Self {
        ColumnDef::new(name, ColumnType::normalize(type_name))
    }
}

/// Look a column up by exact name. Names are matched verbatim — the grammar
/// does not trim what is between the brackets, so `[ s ]` and `[s]` are
/// different columns.
pub(crate) fn find_column<'a>(columns: &'a [ColumnDef], name: &str) -> Option<&'a ColumnDef> {
    columns.iter().find(|c| c.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_number_types() {
        for name in ["INT32", "INT64", "DOUBLE", "DECIMAL", "NUMBER"] {
            assert_eq!(ColumnType::normalize(name), ColumnType::Number, "{name}");
            assert_eq!(
                ColumnType::normalize(&name.to_lowercase()),
                ColumnType::Number,
                "{name} lowercase"
            );
        }
    }

    #[test]
    fn normalizes_string_types() {
        for name in ["TEXT", "STRING", "ICON", "text", "Icon"] {
            assert_eq!(ColumnType::normalize(name), ColumnType::String, "{name}");
        }
    }

    #[test]
    fn normalizes_boolean_date_and_enum() {
        assert_eq!(ColumnType::normalize("BOOLEAN"), ColumnType::Boolean);
        assert_eq!(ColumnType::normalize("boolean"), ColumnType::Boolean);
        assert_eq!(ColumnType::normalize("DATE"), ColumnType::Date);
        assert_eq!(ColumnType::normalize("DATETIME"), ColumnType::Date);
        assert_eq!(ColumnType::normalize("datetime"), ColumnType::Date);
        assert_eq!(ColumnType::normalize("ENUM"), ColumnType::Enum);
        assert_eq!(ColumnType::normalize("enum"), ColumnType::Enum);
    }

    #[test]
    fn unknown_types_fall_back_to_string() {
        for name in ["GUID", "", "Labels", "Link", "whatever"] {
            assert_eq!(ColumnType::normalize(name), ColumnType::String, "{name}");
        }
    }

    #[test]
    fn default_is_string() {
        assert_eq!(ColumnType::default(), ColumnType::String);
    }

    #[test]
    fn type_names_are_lowercase() {
        assert_eq!(ColumnType::String.as_str(), "string");
        assert_eq!(ColumnType::Number.as_str(), "number");
        assert_eq!(ColumnType::Boolean.as_str(), "boolean");
        assert_eq!(ColumnType::Date.as_str(), "date");
        assert_eq!(ColumnType::Enum.as_str(), "enum");
    }

    #[test]
    fn normalized_constructor_maps_the_type() {
        let col = ColumnDef::normalized("age", "INT64");
        assert_eq!(col.name, "age");
        assert_eq!(col.column_type, ColumnType::Number);
    }

    #[test]
    fn lookup_is_exact() {
        let cols = vec![
            ColumnDef::new("age", ColumnType::Number),
            ColumnDef::new(" s ", ColumnType::String),
        ];
        assert_eq!(find_column(&cols, "age").unwrap().name, "age");
        assert_eq!(find_column(&cols, " s ").unwrap().name, " s ");
        assert!(find_column(&cols, "s").is_none());
        assert!(find_column(&cols, "AGE").is_none());
    }

    #[test]
    fn serializes_type_as_lowercase_string() {
        let col = ColumnDef::new("age", ColumnType::Number);
        assert_eq!(
            serde_json::to_value(&col).unwrap(),
            serde_json::json!({"name": "age", "type": "number"})
        );
    }
}
