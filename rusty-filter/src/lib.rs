//! Filter query grammar, AST, validator, evaluator and printer.
//!
//! Rusty's `DataTable` renders the same filter query editor Ivy uses, so a query
//! typed in the browser has to mean the same thing on the server. This crate is
//! the Rust half: it parses a query string into the same AST, validates it
//! against a column schema, evaluates it against rows, and prints it back to a
//! canonical string suitable for a cache key.
//!
//! # Where the grammar comes from
//!
//! The grammar is not invented here. It is fixed by `docs/grammar/Filters.g4` in
//! `Ivy-Interactive/Ivy-Query-Editor` and by the shipped `filter-query-editor`
//! bundle under `src/frontend/node_modules/`. Behaviour was matched against
//! version **2.2.0** by calling `parseQuery`, `validateFilters`, `formatQuery`
//! and `evaluateFilter` over roughly a hundred inputs. Nothing in this crate
//! links against an ANTLR runtime: the lexer and the recursive-descent parser
//! are hand-written, and the only dependencies are `serde` and `serde_json`.
//!
//! # Getting started
//!
//! ```
//! use rusty_filter::{parse_query, retain_matching, ColumnDef, ColumnType};
//!
//! let columns = vec![
//!     ColumnDef::new("age", ColumnType::Number),
//!     ColumnDef::new("name", ColumnType::String),
//! ];
//! let result = parse_query("[age] > 30 AND [name] starts with \"A\"", &columns);
//! let filter = result.filters.expect("the query is valid");
//!
//! let rows = vec![
//!     serde_json::json!({"age": 41, "name": "Ada"}),
//!     serde_json::json!({"age": 41, "name": "Bob"}),
//!     serde_json::json!({"age": 12, "name": "Ann"}),
//! ];
//! let kept = retain_matching(&filter, rows, &columns);
//! assert_eq!(kept.len(), 1);
//! ```
//!
//! An invalid query returns its errors instead:
//!
//! ```
//! use rusty_filter::{parse_query, ColumnDef, ColumnType};
//!
//! let columns = vec![ColumnDef::new("age", ColumnType::Number)];
//! let result = parse_query("[age] contains \"3\"", &columns);
//! assert!(result.has_errors());
//! assert_eq!(
//!     result.errors()[0].message,
//!     "Operator 'contains' is not compatible with type 'number'"
//! );
//! ```
//!
//! # Known divergences from the reference
//!
//! Four, all deliberate, each restated where it applies:
//!
//! * **Error spans are byte offsets, not UTF-16 code units.** The two agree for
//!   ASCII queries and part company after a non-BMP character. Anything
//!   highlighting a span from a Rust-side error has to account for that.
//! * **A syntax error stops the parse.** ANTLR recovers and can report a
//!   cascade; here the first error is the only one. Semantic errors are still
//!   reported in full, one per offending condition.
//! * **Evaluation uses the normalized column type.** The reference validates
//!   against a normalized type but evaluates against the raw one, so a column
//!   declared `INT32` passes validation for `>` and then silently matches
//!   nothing. [`ColumnDef`] stores a normalized [`ColumnType`], so the same
//!   filter matches here. See [`eval`].
//! * **A datetime with no `Z` is read as UTC.** The browser reads it in the
//!   machine's local zone, which would make server-side filtering depend on the
//!   server's timezone. See [`eval`].
//!
//! # Module layout
//!
//! [`lexer`] turns a query into tokens, [`parser`] turns tokens into the
//! [`ast`], [`validate`] checks that AST against a [`column`] schema,
//! [`eval`] runs it over rows, and [`print`] turns it back into text.

pub mod ast;
pub mod column;
pub mod eval;
pub mod lexer;
pub mod parser;
pub mod print;
pub mod validate;

pub use ast::{Condition, Filter, FilterFunction, FilterGroup, LogicalOp};
pub use column::{ColumnDef, ColumnType};
pub use eval::{count_matches, evaluate, retain_matching};
pub use parser::{parse_query, parse_query_unchecked, ErrorSeverity, ParseError, ParseResult};
pub use print::{canonical_key, to_query_string};
pub use validate::validate_filter_group;
