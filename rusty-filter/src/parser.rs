//! Recursive-descent parser for the filter grammar, one function per parser rule.
//!
//! Precedence, from loosest to tightest: `OR`, `AND`, `NOT`, primary. The
//! AST-shaping rules are those of `dist/parser/ASTBuilder.js` and are reproduced
//! rather than simplified — in particular parentheses are never collapsed and
//! `NOT` *toggles* negation instead of forcing it true.
//!
//! # How error messages are worded
//!
//! ANTLR has four error shapes, and which one appears is decided mechanically.
//! Reproducing that decision is the only way a Rust-side message can read like
//! the browser's, so each rule below is implemented rather than approximated:
//!
//! * `extraneous input 'X' expecting T` — deleting the current token would let
//!   the match succeed.
//! * `missing T at 'X'` — the current token could legally *follow* the token
//!   that is missing, so one is assumed to have been left out.
//! * `mismatched input 'X' expecting T` — neither repair works.
//! * `no viable alternative at input 'X'` — a decision needing more than one
//!   token of lookahead ran out of alternatives. `X` is every token's raw text
//!   from the start of that decision onwards, run together.
//!
//! Deletion is tried before insertion, which is why `[age] = --5` blames the
//! extra sign instead of reporting a missing digit. Messages quote the raw
//! lexeme, so `[age] GREATER 5` complains about `GREATER`, not `greater`.
//!
//! # Divergences from the reference
//!
//! * `start` and `end` in [`ParseError`] are **byte** offsets into the input;
//!   the TypeScript version reports UTF-16 code units. They agree for ASCII
//!   input and differ once a multi-byte character appears before the error.
//! * On a syntax error this parser stops and reports **one** error. The ANTLR
//!   reference recovers and can emit a cascade of several errors for one input;
//!   matching that cascade is out of scope. The first error agrees.

use serde::{Deserialize, Serialize};

use crate::ast::{Condition, Filter, FilterFunction, FilterGroup};
use crate::column::ColumnDef;
use crate::lexer::{tokenize, SpannedToken, Token};
use crate::validate::validate_filter_group;

/// How serious a [`ParseError`] is. The reference emits `error` for everything
/// this crate produces; `Warning` exists to mirror the TypeScript union.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorSeverity {
    #[default]
    Error,
    Warning,
}

/// A syntax or semantic error, with the byte span it applies to.
///
/// Semantic errors carry `start: 0` and `end: 0` because the reference
/// validator has no position information to report — that is faithful, not a
/// defect to be "fixed".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseError {
    pub message: String,
    pub start: usize,
    pub end: usize,
    pub severity: ErrorSeverity,
}

impl ParseError {
    pub fn new(message: impl Into<String>, start: usize, end: usize) -> Self {
        ParseError {
            message: message.into(),
            start,
            end,
            severity: ErrorSeverity::Error,
        }
    }
}

/// The result of parsing: filters *or* errors, never both, mirroring
/// `dist/types/parser.d.ts` and the early returns of `parseQuery`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ParseResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<FilterGroup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ParseError>>,
}

impl ParseResult {
    fn ok(filters: FilterGroup) -> Self {
        ParseResult {
            filters: Some(filters),
            errors: None,
        }
    }

    fn failed(errors: Vec<ParseError>) -> Self {
        ParseResult {
            filters: None,
            errors: Some(errors),
        }
    }

    /// Whether the parse produced errors.
    pub fn has_errors(&self) -> bool {
        self.errors.as_ref().is_some_and(|e| !e.is_empty())
    }

    /// The errors, or an empty slice on success.
    pub fn errors(&self) -> &[ParseError] {
        self.errors.as_deref().unwrap_or(&[])
    }
}

/// Parse `query` and validate it against `columns`.
///
/// Whitespace-only and empty input yield an empty `AND` group with no errors.
/// Syntax errors are reported before semantic ones, and semantic validation only
/// runs on a syntactically valid query — the same order `parseQuery` uses.
pub fn parse_query(query: &str, columns: &[ColumnDef]) -> ParseResult {
    let group = match parse_query_unchecked(query) {
        Ok(group) => group,
        Err(errors) => return ParseResult::failed(errors),
    };
    let semantic = validate_filter_group(&group, columns);
    if semantic.is_empty() {
        ParseResult::ok(group)
    } else {
        ParseResult::failed(semantic)
    }
}

/// Parse `query` without semantic validation, so no column schema is needed.
pub fn parse_query_unchecked(query: &str) -> Result<FilterGroup, Vec<ParseError>> {
    if query.trim().is_empty() {
        return Ok(FilterGroup::default());
    }
    let tokens = tokenize(query).map_err(|e| vec![ParseError::new(e.message, e.start, e.end)])?;
    let mut parser = Parser::new(&tokens);
    let group = parser.formula().map_err(|e| vec![e])?;
    Ok(group)
}

/// The result of visiting one rule: either a bare filter or a whole group,
/// exactly the untagged union `ASTBuilder`'s visitors return.
#[derive(Debug, Clone)]
enum Node {
    Filter(Filter),
    Group(FilterGroup),
}

impl Node {
    /// Wrap into a single `Filter`, as `visitGroup` does when the inner
    /// expression came back as a group.
    fn into_filter(self) -> Filter {
        match self {
            Node::Filter(f) => f,
            Node::Group(g) => Filter::from_group(g),
        }
    }

    /// Lift into a `FilterGroup`, as `visitAndExpr` does for a single arm.
    fn into_group(self) -> FilterGroup {
        match self {
            Node::Filter(f) => FilterGroup::and(vec![f]),
            Node::Group(g) => g,
        }
    }

    /// Toggle negation on whichever variant this is. `visitUnaryExpr` sets
    /// `negate = !negate` on the visited node, and a group node carries its own
    /// `negate` only once it has been wrapped in a filter.
    fn toggle_negate(self) -> Node {
        match self {
            Node::Filter(mut f) => {
                f.negate = Some(!f.is_negated());
                Node::Filter(f)
            }
            Node::Group(g) => {
                // A raw group has no `negate` field of its own, so the reference
                // sets one on the object it is holding. Wrapping it in a filter
                // is the faithful Rust equivalent: `NOT (a AND b)` becomes a
                // group filter with `negate: true`.
                let mut f = Filter::from_group(g);
                f.negate = Some(true);
                Node::Filter(f)
            }
        }
    }
}

struct Parser<'a> {
    tokens: &'a [SpannedToken],
    pos: usize,
    /// How many `(` are currently open. Only the error wording reads this: a
    /// token's legal followers depend on the enclosing groups, so `)` follows a
    /// completed primary only inside one and `<EOF>` only outside them all.
    depth: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [SpannedToken]) -> Self {
        Parser {
            tokens,
            pos: 0,
            depth: 0,
        }
    }

    fn peek(&self) -> &'a Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)].token
    }

    fn peek_at(&self, offset: usize) -> &'a Token {
        let idx = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[idx].token
    }

    fn current(&self) -> &'a SpannedToken {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> &'a SpannedToken {
        let tok = self.current();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn eat(&mut self, expected: &Token) -> bool {
        if self.peek() == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        let tok = self.current();
        ParseError::new(message, tok.start, tok.end)
    }

    /// Match a single expected token, or word the failure as ANTLR would.
    ///
    /// `expecting` names the token, `matches` recognises it (needed because
    /// `STRING` and `DIGITS` carry payloads), and `follows` says whether a token
    /// may legally appear *after* the expected one — which is what licenses the
    /// `missing` wording. Deletion is tried before insertion.
    fn expect(
        &mut self,
        expecting: &str,
        matches: fn(&Token) -> bool,
        follows: Follows,
    ) -> Result<&'a SpannedToken, ParseError> {
        if matches(self.peek()) {
            return Ok(self.advance());
        }
        let quoted = self.current().describe();
        if matches(self.peek_at(1)) {
            return Err(self.error(format!("extraneous input '{quoted}' expecting {expecting}")));
        }
        if follows(self.peek(), self.depth) {
            return Err(self.error(format!("missing {expecting} at '{quoted}'")));
        }
        Err(self.error(format!("mismatched input '{quoted}' expecting {expecting}")))
    }

    /// The failure for a position that expects one of a *set* of tokens.
    ///
    /// No `missing` wording here: with several candidates the reference never
    /// picks one to insert, so only deletion is offered.
    fn expected_set(&self, expecting: &str, matches: fn(&Token) -> bool) -> ParseError {
        let quoted = self.current().describe();
        if matches(self.peek_at(1)) {
            self.error(format!("extraneous input '{quoted}' expecting {expecting}"))
        } else {
            self.error(format!("mismatched input '{quoted}' expecting {expecting}"))
        }
    }

    /// The failure for a decision that ran out of alternatives.
    ///
    /// The quoted text is every token's raw spelling from `from` through the
    /// token where prediction gave up, run together with no separators — so
    /// `[age] not blank` reports `'[age]notblank'`. The span is that last
    /// token's.
    fn no_viable(&self, from: usize, failed_at: usize) -> ParseError {
        let last = failed_at.min(self.tokens.len() - 1);
        let text: String = self.tokens[from..=last]
            .iter()
            .map(|t| t.text.as_str())
            .collect();
        let tok = &self.tokens[last];
        ParseError::new(
            format!("no viable alternative at input '{text}'"),
            tok.start,
            tok.end,
        )
    }

    /// `formula : expr EOF`
    fn formula(&mut self) -> Result<FilterGroup, ParseError> {
        let node = self.expr()?;
        self.expect("<EOF>", is_eof, never)?;
        Ok(node.into_group())
    }

    /// `expr : orExpr`
    fn expr(&mut self) -> Result<Node, ParseError> {
        self.or_expr()
    }

    /// `orExpr : andExpr (OR andExpr)*`
    ///
    /// A single arm is returned unchanged. Several arms produce an `OR` group in
    /// which each arm that came back as a group is spliced in bare when it holds
    /// exactly one filter, and wrapped as `{group: ...}` otherwise.
    fn or_expr(&mut self) -> Result<Node, ParseError> {
        let first = self.and_expr()?;
        if self.peek() != &Token::Or {
            return Ok(first);
        }
        let mut arms = vec![first];
        while self.eat(&Token::Or) {
            arms.push(self.and_expr()?);
        }
        let mut filters = Vec::with_capacity(arms.len());
        for arm in arms {
            match arm {
                Node::Group(group) => {
                    if group.filters.len() == 1 {
                        filters.push(group.filters.into_iter().next().expect("length checked"));
                    } else {
                        filters.push(Filter::from_group(group));
                    }
                }
                Node::Filter(f) => filters.push(f),
            }
        }
        Ok(Node::Group(FilterGroup::or(filters)))
    }

    /// `andExpr : unaryExpr (AND unaryExpr)*`
    ///
    /// A single arm that is already a group is returned as-is; a single filter is
    /// wrapped in an `AND` group. Several arms are pushed as-is with no splicing.
    fn and_expr(&mut self) -> Result<Node, ParseError> {
        let first = self.unary_expr()?;
        if self.peek() != &Token::And {
            return Ok(Node::Group(first.into_group()));
        }
        let mut filters = vec![first.into_filter()];
        while self.eat(&Token::And) {
            filters.push(self.unary_expr()?.into_filter());
        }
        Ok(Node::Group(FilterGroup::and(filters)))
    }

    /// `unaryExpr : NOT unaryExpr | primary`
    fn unary_expr(&mut self) -> Result<Node, ParseError> {
        if self.eat(&Token::Not) {
            let inner = self.unary_expr()?;
            return Ok(inner.toggle_negate());
        }
        self.primary()
    }

    /// `primary : group | comparison | textOperation | existenceOperation`
    ///
    /// The three field-led alternatives are chosen by looking past the field
    /// reference and an optional `NOT`. When no alternative survives that
    /// lookahead the failure is a `no viable alternative` quoting from the field,
    /// because the field is where all three alternatives begin.
    fn primary(&mut self) -> Result<Node, ParseError> {
        if self.peek() == &Token::LParen {
            return self.group();
        }
        let field = self.pos;
        let Token::Field(column) = self.peek().clone() else {
            return Err(self.expected_set("{FIELD, NOT, '('}", starts_primary));
        };
        match self.peek_at(1) {
            Token::Contains | Token::Starts | Token::Ends => {
                self.advance();
                self.text_operation(column)
            }
            Token::Is => {
                self.advance();
                self.existence_operation(column, field)
            }
            // After `NOT` only negated equality and the text operators remain.
            Token::Not => match self.peek_at(2) {
                Token::Equals | Token::Equal => {
                    self.advance();
                    self.comparison(column)
                }
                Token::Contains | Token::Starts | Token::Ends => {
                    self.advance();
                    self.text_operation(column)
                }
                _ => Err(self.no_viable(field, self.pos + 2)),
            },
            next if starts_comp_op(next) => {
                self.advance();
                self.comparison(column)
            }
            _ => Err(self.no_viable(field, self.pos + 1)),
        }
    }

    /// `group : LPAREN expr RPAREN`
    ///
    /// Parentheses are never collapsed: the inner group is wrapped as a group
    /// filter, so `(([age] > 1))` keeps both levels.
    fn group(&mut self) -> Result<Node, ParseError> {
        self.advance(); // LPAREN
        self.depth += 1;
        let inner = self.expr()?;
        // The `)` this rule is about is the one being closed, so its own follow
        // set is that of the *enclosing* group: `([age] > 1` reports a missing
        // `)` at `<EOF>`, while `(([age] > 1)` reports the same at depth 1.
        self.depth -= 1;
        self.expect("')'", is_rparen, ends_primary)?;
        Ok(match inner {
            Node::Group(g) => Node::Filter(Filter::from_group(g)),
            Node::Filter(f) => Node::Filter(f),
        })
    }

    /// `comparison : fieldRef compOp operand`
    ///
    /// `negate` is `Some(true)` for the three not-equal spellings and `None`
    /// otherwise — comparisons never emit `negate: false`.
    fn comparison(&mut self, column: String) -> Result<Node, ParseError> {
        let (function, negate) = self.comp_op(self.pos)?;
        let value = self.operand()?;
        let mut filter = Filter {
            condition: Some(Condition::new(column, function, vec![value])),
            group: None,
            negate: None,
        };
        if negate {
            filter.negate = Some(true);
        }
        Ok(Node::Filter(filter))
    }

    /// `compOp` — returns the mapped function and whether it negates.
    ///
    /// `op_start` is where a `no viable alternative` message should quote from:
    /// the word operators are their own decision, so `[age] greater 5` quotes
    /// `'greater5'` and not the field, while a failure that only becomes visible
    /// here is still attributed to the whole primary.
    fn comp_op(&mut self, op_start: usize) -> Result<(FilterFunction, bool), ParseError> {
        let tok = self.peek().clone();
        match tok {
            Token::Eq | Token::Eq2 => {
                self.advance();
                Ok((FilterFunction::Equals, false))
            }
            Token::Neq => {
                self.advance();
                Ok((FilterFunction::Equals, true))
            }
            Token::Gt => {
                self.advance();
                Ok((FilterFunction::GreaterThan, false))
            }
            Token::Ge => {
                self.advance();
                Ok((FilterFunction::GreaterThanOrEqual, false))
            }
            Token::Lt => {
                self.advance();
                Ok((FilterFunction::LessThan, false))
            }
            Token::Le => {
                self.advance();
                Ok((FilterFunction::LessThanOrEqual, false))
            }
            Token::Equals => {
                self.advance();
                Ok((FilterFunction::Equals, false))
            }
            // `NOT EQUALS` and `NOT EQUAL` both mean negated equality. A bare
            // `EQUAL` without `NOT` is not a `compOp` alternative, which is why
            // `primary` never routes one here.
            Token::Not => {
                self.advance();
                self.advance(); // EQUALS or EQUAL, checked by `primary`
                Ok((FilterFunction::Equals, true))
            }
            Token::Greater | Token::Less => {
                let is_greater = tok == Token::Greater;
                let word_start = self.pos;
                self.advance();
                if self.peek() != &Token::Than {
                    // `greater`/`less` without `than` exhausts the alternatives
                    // for this decision, so the message quotes from the word.
                    return Err(self.no_viable(word_start, self.pos));
                }
                self.advance();
                // The optional `OR EQUAL` tail. `OR EQUALS` (plural) is rejected
                // by the grammar, which spells this alternative `OR EQUAL`.
                if self.eat(&Token::Or) {
                    self.expect("EQUAL", is_equal, operand_follows)?;
                    Ok((
                        if is_greater {
                            FilterFunction::GreaterThanOrEqual
                        } else {
                            FilterFunction::LessThanOrEqual
                        },
                        false,
                    ))
                } else {
                    Ok((
                        if is_greater {
                            FilterFunction::GreaterThan
                        } else {
                            FilterFunction::LessThan
                        },
                        false,
                    ))
                }
            }
            // Unreachable: `primary` only routes a `compOp` start here.
            _ => Err(self.no_viable(op_start, self.pos)),
        }
    }

    /// `textOperation : fieldRef NOT? textOp stringLiteral`
    ///
    /// `negate` is **always** `Some`, `false` included.
    fn text_operation(&mut self, column: String) -> Result<Node, ParseError> {
        let has_not = self.eat(&Token::Not);
        let function = self.text_op()?;
        let token = self.expect("STRING", is_string, ends_primary)?;
        let Token::Str(value) = &token.token else {
            unreachable!("`expect` matched a STRING");
        };
        let filter = Filter {
            condition: Some(Condition::new(
                column,
                function,
                vec![serde_json::Value::String(value.clone())],
            )),
            group: None,
            negate: Some(has_not),
        };
        Ok(Node::Filter(filter))
    }

    /// `textOp : CONTAINS | STARTS WITH | ENDS WITH`
    fn text_op(&mut self) -> Result<FilterFunction, ParseError> {
        match self.peek().clone() {
            Token::Contains => {
                self.advance();
                Ok(FilterFunction::Contains)
            }
            Token::Starts => {
                self.advance();
                self.expect("WITH", is_with, string_follows)?;
                Ok(FilterFunction::StartsWith)
            }
            // Unreachable for anything but `ENDS`: `primary` chose this rule.
            _ => {
                self.advance();
                self.expect("WITH", is_with, string_follows)?;
                Ok(FilterFunction::EndsWith)
            }
        }
    }

    /// `existenceOperation : fieldRef IS BLANK | fieldRef IS NOT BLANK`
    ///
    /// Produces `IsBlank` or `IsNotBlank` with empty `args` and no `negate`.
    /// `field` is where a `no viable alternative` quotes from, since `[age] is 5`
    /// abandons the whole primary rather than just this rule.
    fn existence_operation(&mut self, column: String, field: usize) -> Result<Node, ParseError> {
        self.advance(); // IS
        let negated = self.eat(&Token::Not);
        if self.peek() != &Token::Blank {
            // `IS NOT` has committed to this alternative, so a missing `BLANK`
            // is reported as such. A bare `IS` followed by junk has not, and
            // fails the primary decision instead.
            if negated {
                self.expect("BLANK", is_blank, ends_primary)?;
            } else {
                return Err(self.no_viable(field, self.pos));
            }
        }
        self.advance();
        let function = if negated {
            FilterFunction::IsNotBlank
        } else {
            FilterFunction::IsBlank
        };
        Ok(Node::Filter(Filter::condition(column, function, vec![])))
    }

    /// `operand : number | stringLiteral | booleanLiteral`
    fn operand(&mut self) -> Result<serde_json::Value, ParseError> {
        match self.peek().clone() {
            Token::Str(s) => {
                self.advance();
                Ok(serde_json::Value::String(s))
            }
            Token::True => {
                self.advance();
                Ok(serde_json::Value::Bool(true))
            }
            Token::False => {
                self.advance();
                Ok(serde_json::Value::Bool(false))
            }
            Token::Sign(_) | Token::Digits(_) => self.number(),
            _ => Err(self.expected_set(OPERAND_SET, starts_operand)),
        }
    }

    /// `number : SIGN? DIGITS (DOT DIGITS)?`
    ///
    /// Both sides of the dot are required, so `.5` and `5.` are errors. Since
    /// whitespace is skipped between tokens, `1 . 5` is `1.5`.
    fn number(&mut self) -> Result<serde_json::Value, ParseError> {
        let negative = match self.peek() {
            Token::Sign(c) => {
                let neg = *c == '-';
                self.advance();
                Some(neg)
            }
            _ => None,
        };
        // A sign with no digits ends the primary, so the follow set is the
        // primary's: `[age] = -` reports a missing DIGITS at `<EOF>`, while
        // `[age] = - "x"` is a mismatch because a string cannot follow either.
        let int_token = self.expect("DIGITS", is_digits, ends_primary)?;
        let Token::Digits(int_part) = &int_token.token else {
            unreachable!("`expect` matched DIGITS");
        };
        let mut text = int_part.clone();
        // The fraction is committed to on sight of the dot: the reference's
        // adaptive prediction would look ahead, but every input where the two
        // could differ is a syntax error either way.
        if self.peek() == &Token::Dot {
            self.advance();
            let frac_token = self.expect("DIGITS", is_digits, ends_primary)?;
            let Token::Digits(frac) = &frac_token.token else {
                unreachable!("`expect` matched DIGITS");
            };
            text.push('.');
            text.push_str(frac);
        }
        Ok(js_number(&text, negative == Some(true)))
    }
}

/// The token set an `operand` may start with, spelled as the message spells it.
const OPERAND_SET: &str = "{STRING, TRUE, FALSE, SIGN, DIGITS}";

/// Build the JSON number `parseFloat` would produce for `text`.
///
/// The distinction matters for more than tidiness: `serde_json` remembers
/// whether a number was written as an integer, and `1.0` is not equal to `1`.
/// JavaScript has one number type, so an integral value must come back as an
/// integer or every AST comparison against the frontend's JSON fails.
fn js_number(text: &str, negative: bool) -> serde_json::Value {
    // `parseFloat` on an over-long literal yields `Infinity`, which JSON cannot
    // represent; the reference emits `null` for that arg.
    let Ok(magnitude) = text.parse::<f64>() else {
        return serde_json::Value::Null;
    };
    let value = if negative { -magnitude } else { magnitude };
    if !value.is_finite() {
        return serde_json::Value::Null;
    }
    // `-0` is `0` in JSON, and an integral f64 must serialize without a `.0`.
    if value == value.trunc() && value.abs() < 9.007_199_254_740_992e15 {
        return serde_json::Value::Number((value as i64).into());
    }
    serde_json::Number::from_f64(value)
        .map(serde_json::Value::Number)
        .unwrap_or(serde_json::Value::Null)
}

// The token predicates `expect` and `expected_set` are parameterised over. Each
// is a plain fn so the call sites stay allocation-free.

/// Whether a token may legally follow the one `expect` is looking for, given how
/// many groups are currently open. The depth is what makes `missing STRING at
/// ')'` correct inside a group and `mismatched input ')' expecting STRING`
/// correct outside one.
type Follows = fn(&Token, usize) -> bool;

fn never(_: &Token, _: usize) -> bool {
    false
}

fn is_eof(token: &Token) -> bool {
    token == &Token::Eof
}

fn is_rparen(token: &Token) -> bool {
    token == &Token::RParen
}

fn is_equal(token: &Token) -> bool {
    token == &Token::Equal
}

fn is_with(token: &Token) -> bool {
    token == &Token::With
}

fn is_blank(token: &Token) -> bool {
    token == &Token::Blank
}

fn is_string(token: &Token) -> bool {
    matches!(token, Token::Str(_))
}

fn is_digits(token: &Token) -> bool {
    matches!(token, Token::Digits(_))
}

/// The tokens a `primary` may start with: `{FIELD, NOT, '('}`.
fn starts_primary(token: &Token) -> bool {
    matches!(token, Token::Field(_) | Token::Not | Token::LParen)
}

/// The tokens an `operand` may start with.
fn starts_operand(token: &Token) -> bool {
    matches!(
        token,
        Token::Str(_) | Token::True | Token::False | Token::Sign(_) | Token::Digits(_)
    )
}

/// The tokens a `compOp` may start with.
fn starts_comp_op(token: &Token) -> bool {
    matches!(
        token,
        Token::Eq
            | Token::Eq2
            | Token::Neq
            | Token::Gt
            | Token::Ge
            | Token::Lt
            | Token::Le
            | Token::Equals
            | Token::Greater
            | Token::Less
    )
}

/// What may legally follow a completed `primary`, and so what licenses a
/// `missing` rather than a `mismatched` inside one.
///
/// The depth is load-bearing, because ANTLR computes the follow set from the
/// rule invocation stack rather than from the grammar alone. `)` can only follow
/// while a group is open and `<EOF>` only once every group has closed, which is
/// why `([name] contains` reports `mismatched input '<EOF>' expecting STRING`
/// while the unparenthesised `[name] contains` reports `missing STRING at
/// '<EOF>'`. Measured against the 2.2.0 bundle over `STRING`, `BLANK`, `DIGITS`
/// and `')'` at both depths.
fn ends_primary(token: &Token, depth: usize) -> bool {
    match token {
        Token::And | Token::Or => true,
        Token::RParen => depth > 0,
        Token::Eof => depth == 0,
        _ => false,
    }
}

/// [`starts_operand`] as a [`Follows`]: the operand set does not depend on how
/// many groups are open.
fn operand_follows(token: &Token, _depth: usize) -> bool {
    starts_operand(token)
}

/// [`is_string`] as a [`Follows`], for the `WITH` in `STARTS WITH`.
fn string_follows(token: &Token, _depth: usize) -> bool {
    is_string(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::LogicalOp;
    use serde_json::json;

    fn parse(query: &str) -> FilterGroup {
        parse_query_unchecked(query).unwrap_or_else(|e| panic!("{query:?} failed to parse: {e:?}"))
    }

    fn parse_err(query: &str) -> ParseError {
        parse_query_unchecked(query)
            .map(|g| panic!("{query:?} unexpectedly parsed to {g:?}"))
            .unwrap_err()
            .into_iter()
            .next()
            .expect("at least one error")
    }

    fn cond(column: &str, function: FilterFunction, args: Vec<serde_json::Value>) -> Filter {
        Filter::condition(column, function, args)
    }

    // --- one assertion per row of the measured table -----------------------

    #[test]
    fn measured_greater_than() {
        assert_eq!(
            parse("[age] > 100"),
            FilterGroup::and(vec![cond(
                "age",
                FilterFunction::GreaterThan,
                vec![json!(100)]
            )])
        );
    }

    #[test]
    fn measured_not_equal_is_negated_equals() {
        let expected = FilterGroup::and(vec![
            cond("age", FilterFunction::Equals, vec![json!(5)]).negated(true)
        ]);
        // All three not-equal spellings agree, and none produces a `notEquals`.
        assert_eq!(parse("[age] != 5"), expected);
        assert_eq!(parse("[age] not equals 5"), expected);
        assert_eq!(parse("[age] not equal 5"), expected);
    }

    #[test]
    fn measured_not_contains() {
        assert_eq!(
            parse("[name] not contains \"ab\""),
            FilterGroup::and(vec![cond(
                "name",
                FilterFunction::Contains,
                vec![json!("ab")]
            )
            .negated(true)])
        );
    }

    #[test]
    fn measured_contains_emits_negate_false() {
        let group = parse("[name] contains \"ab\"");
        assert_eq!(group.filters[0].negate, Some(false));
        let value = serde_json::to_value(&group).unwrap();
        assert_eq!(value["filters"][0]["negate"], json!(false));
    }

    #[test]
    fn measured_comparison_omits_negate() {
        let group = parse("[age] > 1");
        assert_eq!(group.filters[0].negate, None);
        let value = serde_json::to_value(&group).unwrap();
        assert!(value["filters"][0].get("negate").is_none());
    }

    #[test]
    fn measured_or_root_with_spliced_and_arm() {
        assert_eq!(
            parse("[age] > 1 AND [n] = \"a\" OR [x] = true"),
            FilterGroup::or(vec![
                Filter::from_group(FilterGroup::and(vec![
                    cond("age", FilterFunction::GreaterThan, vec![json!(1)]),
                    cond("n", FilterFunction::Equals, vec![json!("a")]),
                ])),
                cond("x", FilterFunction::Equals, vec![json!(true)]),
            ])
        );
    }

    #[test]
    fn measured_not_lands_on_the_condition_filter() {
        assert_eq!(
            parse("NOT [age] > 1"),
            FilterGroup::and(vec![cond(
                "age",
                FilterFunction::GreaterThan,
                vec![json!(1)]
            )
            .negated(true)])
        );
    }

    #[test]
    fn measured_not_toggles() {
        let group = parse("not not [age] > 1");
        assert_eq!(group.filters[0].negate, Some(false));
        assert_eq!(parse("NOT NOT NOT [age] > 1").filters[0].negate, Some(true));
    }

    #[test]
    fn measured_parens_are_not_collapsed() {
        assert_eq!(
            parse("(([age] > 1))"),
            FilterGroup::and(vec![Filter::from_group(FilterGroup::and(vec![
                Filter::from_group(FilterGroup::and(vec![cond(
                    "age",
                    FilterFunction::GreaterThan,
                    vec![json!(1)]
                )]))
            ]))])
        );
        // A third level nests a third time.
        let deep = parse("((([age] > 1)))");
        let lvl1 = deep.filters[0].group.as_ref().unwrap();
        let lvl2 = lvl1.filters[0].group.as_ref().unwrap();
        let lvl3 = lvl2.filters[0].group.as_ref().unwrap();
        assert!(lvl3.filters[0].condition.is_some());
    }

    #[test]
    fn measured_empty_and_blank_input() {
        assert_eq!(parse(""), FilterGroup::default());
        assert_eq!(parse("   "), FilterGroup::default());
        assert_eq!(parse("\t\r\n"), FilterGroup::default());
        // and no errors
        assert!(!parse_query("", &[]).has_errors());
        assert!(!parse_query("   ", &[]).has_errors());
    }

    #[test]
    fn measured_whitespace_inside_a_number() {
        for query in ["[age] = 1 . 5", "[age] = 1 .5", "[age] = 1. 5"] {
            assert_eq!(
                parse(query).filters[0].condition.as_ref().unwrap().args[0],
                json!(1.5),
                "{query}"
            );
        }
    }

    #[test]
    fn measured_leading_zeros_are_dropped() {
        assert_eq!(
            parse("[age] = 007").filters[0]
                .condition
                .as_ref()
                .unwrap()
                .args[0],
            json!(7)
        );
        assert_eq!(
            parse("[age] = 000000123").filters[0]
                .condition
                .as_ref()
                .unwrap()
                .args[0],
            json!(123)
        );
    }

    #[test]
    fn measured_single_quotes_are_not_string_delimiters() {
        assert_eq!(
            parse_err("[name] = 'x'").message,
            "token recognition error at: '''"
        );
    }

    #[test]
    fn measured_exponent_is_rejected() {
        assert_eq!(
            parse_err("[age] = 1e5").message,
            "token recognition error at: 'e5'"
        );
    }

    #[test]
    fn measured_both_sides_of_the_dot_are_required() {
        // `.5` — the dot is not a valid operand start.
        assert!(parse_err("[age] = .5").message.contains("expecting"));
        // `5.` — the fraction digits are missing.
        assert_eq!(parse_err("[age] = 5.").message, "missing DIGITS at '<EOF>'");
        assert_eq!(
            parse_err("[age] = 1 .").message,
            "missing DIGITS at '<EOF>'"
        );
    }

    #[test]
    fn measured_nested_bracket_is_a_legal_column_name() {
        assert_eq!(
            parse("[a[b] = \"x\"").filters[0]
                .condition
                .as_ref()
                .unwrap()
                .column,
            "a[b"
        );
    }

    #[test]
    fn measured_single_space_is_a_legal_column_name() {
        assert_eq!(
            parse("[ ] = \"x\"").filters[0]
                .condition
                .as_ref()
                .unwrap()
                .column,
            " "
        );
        assert!(parse_query_unchecked("[] = \"x\"").is_err());
    }

    // --- operator spellings ------------------------------------------------

    #[test]
    fn symbolic_operators_map_to_functions() {
        let cases = [
            ("=", FilterFunction::Equals),
            ("==", FilterFunction::Equals),
            (">", FilterFunction::GreaterThan),
            (">=", FilterFunction::GreaterThanOrEqual),
            ("<", FilterFunction::LessThan),
            ("<=", FilterFunction::LessThanOrEqual),
        ];
        for (op, expected) in cases {
            let group = parse(&format!("[age] {op} 5"));
            assert_eq!(
                group.filters[0].condition.as_ref().unwrap().function,
                expected,
                "{op}"
            );
        }
    }

    #[test]
    fn word_operators_map_to_functions() {
        let cases = [
            ("equals", FilterFunction::Equals),
            ("greater than", FilterFunction::GreaterThan),
            ("greater than or equal", FilterFunction::GreaterThanOrEqual),
            ("less than", FilterFunction::LessThan),
            ("less than or equal", FilterFunction::LessThanOrEqual),
            ("GREATER THAN OR EQUAL", FilterFunction::GreaterThanOrEqual),
        ];
        for (op, expected) in cases {
            let group = parse(&format!("[age] {op} 5"));
            assert_eq!(
                group.filters[0].condition.as_ref().unwrap().function,
                expected,
                "{op}"
            );
        }
    }

    #[test]
    fn bare_equal_without_not_is_rejected() {
        assert_eq!(
            parse_err("[age] equal 5").message,
            "no viable alternative at input '[age]equal'"
        );
    }

    #[test]
    fn plural_or_equals_is_rejected() {
        // The grammar spells this tail `OR EQUAL`, singular.
        assert_eq!(
            parse_err("[age] greater than or equals 5").message,
            "mismatched input 'equals' expecting EQUAL"
        );
        assert_eq!(
            parse_err("[age] less than or equals 5").message,
            "mismatched input 'equals' expecting EQUAL"
        );
    }

    #[test]
    fn text_operators_and_existence_operators() {
        assert_eq!(
            parse("[name] starts with \"a\"").filters[0]
                .condition
                .as_ref()
                .unwrap()
                .function,
            FilterFunction::StartsWith
        );
        assert_eq!(
            parse("[name] ends with \"a\"").filters[0]
                .condition
                .as_ref()
                .unwrap()
                .function,
            FilterFunction::EndsWith
        );
        assert_eq!(
            parse("[name] is blank"),
            FilterGroup::and(vec![cond("name", FilterFunction::IsBlank, vec![])])
        );
        assert_eq!(
            parse("[name] is not blank"),
            FilterGroup::and(vec![cond("name", FilterFunction::IsNotBlank, vec![])])
        );
    }

    #[test]
    fn existence_operations_carry_no_negate_key() {
        for query in ["[name] is blank", "[name] is not blank"] {
            let group = parse(query);
            assert_eq!(group.filters[0].negate, None, "{query}");
            assert!(group.filters[0].condition.as_ref().unwrap().args.is_empty());
        }
    }

    #[test]
    fn negated_text_operations_always_emit_negate() {
        for (query, expected) in [
            ("[name] contains \"a\"", false),
            ("[name] not contains \"a\"", true),
            ("[name] starts with \"a\"", false),
            ("[name] not starts with \"a\"", true),
            ("[name] ends with \"a\"", false),
            ("[name] not ends with \"a\"", true),
        ] {
            assert_eq!(parse(query).filters[0].negate, Some(expected), "{query}");
        }
    }

    #[test]
    fn text_operation_rejects_a_non_string_argument() {
        assert_eq!(
            parse_err("[name] contains 5").message,
            "mismatched input '5' expecting STRING"
        );
    }

    // --- structural shaping ------------------------------------------------

    #[test]
    fn three_arm_and_stays_flat() {
        let group = parse("[age] > 1 AND [age] < 5 AND [age] != 3");
        assert_eq!(group.op, LogicalOp::And);
        assert_eq!(group.filters.len(), 3);
        assert!(group.filters.iter().all(|f| f.condition.is_some()));
        assert_eq!(group.filters[2].negate, Some(true));
    }

    #[test]
    fn three_arm_or_stays_flat() {
        let group = parse("[age] > 1 OR [age] < 5 OR [age] != 3");
        assert_eq!(group.op, LogicalOp::Or);
        assert_eq!(group.filters.len(), 3);
        assert!(group.filters.iter().all(|f| f.condition.is_some()));
    }

    #[test]
    fn or_splices_single_filter_arms_and_wraps_the_rest() {
        // Middle arm has two filters, so it stays wrapped; the outer arms are bare.
        let group = parse("[age] > 1 OR [age] < 2 AND [age] > 3 OR [age] < 4");
        assert_eq!(group.op, LogicalOp::Or);
        assert_eq!(group.filters.len(), 3);
        assert!(group.filters[0].condition.is_some());
        assert_eq!(
            group.filters[1].group.as_ref().map(|g| g.filters.len()),
            Some(2)
        );
        assert!(group.filters[2].condition.is_some());
    }

    #[test]
    fn an_explicit_group_arm_of_or_keeps_its_wrapper() {
        // `([age] > 1)` is a *group filter* inside a one-filter AND group, so the
        // splice hands the group filter through rather than unwrapping the paren.
        let group = parse("([age] > 1) OR [age] < 2");
        assert_eq!(group.op, LogicalOp::Or);
        assert_eq!(
            group.filters[0]
                .group
                .as_ref()
                .map(|g| (g.op, g.filters.len())),
            Some((LogicalOp::And, 1))
        );
        assert!(group.filters[1].condition.is_some());
    }

    #[test]
    fn a_group_inside_and_is_pushed_as_is() {
        let group = parse("([age] > 1 OR [age] < 0) AND [name] = \"a\"");
        assert_eq!(group.op, LogicalOp::And);
        assert_eq!(group.filters.len(), 2);
        assert_eq!(
            group.filters[0].group.as_ref().map(|g| g.op),
            Some(LogicalOp::Or)
        );
        assert!(group.filters[1].condition.is_some());
    }

    #[test]
    fn not_before_a_group_negates_the_group_filter() {
        let group = parse("NOT ([age] > 1 AND [age] < 5)");
        assert_eq!(group.filters.len(), 1);
        assert_eq!(group.filters[0].negate, Some(true));
        assert_eq!(
            group.filters[0].group.as_ref().map(|g| g.filters.len()),
            Some(2)
        );
    }

    #[test]
    fn not_before_a_group_toggles_too() {
        let group = parse("NOT NOT ([age] > 1)");
        assert_eq!(group.filters[0].negate, Some(false));
        assert!(group.filters[0].group.is_some());
    }

    #[test]
    fn not_inside_an_and_chain() {
        let group = parse("[age] > 1 AND NOT [age] < 5");
        assert_eq!(group.filters.len(), 2);
        assert_eq!(group.filters[0].negate, None);
        assert_eq!(group.filters[1].negate, Some(true));
    }

    // --- syntax errors -----------------------------------------------------

    #[test]
    fn trailing_operator_with_no_operand_is_rejected() {
        assert_eq!(
            parse_err("[age] >").message,
            "mismatched input '<EOF>' expecting {STRING, TRUE, FALSE, SIGN, DIGITS}"
        );
        assert_eq!(
            parse_err("[age] equals").message,
            "mismatched input '<EOF>' expecting {STRING, TRUE, FALSE, SIGN, DIGITS}"
        );
    }

    #[test]
    fn dangling_and_is_rejected() {
        assert_eq!(
            parse_err("[age] > 1 AND").message,
            "mismatched input '<EOF>' expecting {FIELD, NOT, '('}"
        );
        assert_eq!(
            parse_err("[age] > 1 OR").message,
            "mismatched input '<EOF>' expecting {FIELD, NOT, '('}"
        );
    }

    #[test]
    fn trailing_extra_literal_is_rejected() {
        let e = parse_err("[age] > 1 1");
        assert_eq!(e.message, "extraneous input '1' expecting <EOF>");
        assert_eq!((e.start, e.end), (10, 11));
    }

    #[test]
    fn a_second_operator_is_rejected() {
        // `mismatched`, not `extraneous`: nothing may follow `<EOF>`, so deleting
        // the offending token would not let the match succeed either.
        assert_eq!(
            parse_err("[age] = 5 = 5").message,
            "mismatched input '=' expecting <EOF>"
        );
        assert_eq!(
            parse_err("[age] = 1.5.5").message,
            "mismatched input '.' expecting <EOF>"
        );
        // A second *literal* is deletable, because a literal is what `<EOF>`
        // would follow — hence `extraneous` for this one.
        assert_eq!(
            parse_err("[age] > 1 1").message,
            "extraneous input '1' expecting <EOF>"
        );
    }

    #[test]
    fn spaced_two_character_operator_is_rejected() {
        for query in ["[age] > = 5", "[age] < = 5", "[age] = = 5"] {
            let e = parse_err(query);
            assert!(
                e.message
                    .contains("expecting {STRING, TRUE, FALSE, SIGN, DIGITS}"),
                "{query}: {}",
                e.message
            );
        }
    }

    #[test]
    fn double_sign_is_rejected() {
        // Deletion is tried first and the second `-` is followed by a digit, so
        // the extra sign is blamed rather than a digit called missing.
        assert_eq!(
            parse_err("[age] = --5").message,
            "extraneous input '-' expecting DIGITS"
        );
        assert_eq!(
            parse_err("[age] = -.5").message,
            "extraneous input '.' expecting DIGITS"
        );
    }

    #[test]
    fn a_bare_literal_or_field_is_rejected() {
        assert_eq!(
            parse_err("5").message,
            "mismatched input '5' expecting {FIELD, NOT, '('}"
        );
        assert_eq!(
            parse_err("()").message,
            "mismatched input ')' expecting {FIELD, NOT, '('}"
        );
    }

    #[test]
    fn a_field_with_no_operator_quotes_from_the_field() {
        // The three field-led alternatives all begin at the field, so the
        // message runs the tokens together from there rather than naming just
        // the token that failed.
        assert_eq!(
            parse_err("[age]").message,
            "no viable alternative at input '[age]'"
        );
        assert_eq!(
            parse_err("[age] not").message,
            "no viable alternative at input '[age]not'"
        );
        assert_eq!(
            parse_err("[age] not 5").message,
            "no viable alternative at input '[age]not5'"
        );
        assert_eq!(
            parse_err("[age] not blank").message,
            "no viable alternative at input '[age]notblank'"
        );
        assert_eq!(
            parse_err("[age] is").message,
            "no viable alternative at input '[age]is'"
        );
        assert_eq!(
            parse_err("[age] is 5").message,
            "no viable alternative at input '[age]is5'"
        );
        assert_eq!(
            parse_err("[age] not >").message,
            "no viable alternative at input '[age]not>'"
        );
        // Nesting and position do not change where the quoting starts.
        assert_eq!(
            parse_err("([age] equal 5)").message,
            "no viable alternative at input '[age]equal'"
        );
        assert_eq!(
            parse_err("[age] > 1 AND [name] equal \"a\"").message,
            "no viable alternative at input '[name]equal'"
        );
    }

    #[test]
    fn a_word_operator_with_no_than_quotes_from_the_word() {
        // `greater`/`less` open a decision of their own, so the field is not
        // part of the quoted text.
        assert_eq!(
            parse_err("[age] greater 5").message,
            "no viable alternative at input 'greater5'"
        );
        assert_eq!(
            parse_err("[age] less 5").message,
            "no viable alternative at input 'less5'"
        );
        assert_eq!(
            parse_err("[age] greater").message,
            "no viable alternative at input 'greater'"
        );
        assert_eq!(
            parse_err("[age] less").message,
            "no viable alternative at input 'less'"
        );
        assert_eq!(
            parse_err("([age] greater 5)").message,
            "no viable alternative at input 'greater5'"
        );
    }

    #[test]
    fn quoted_text_keeps_the_original_case() {
        assert_eq!(
            parse_err("[age] EQUAL 5").message,
            "no viable alternative at input '[age]EQUAL'"
        );
        assert_eq!(
            parse_err("[age] GREATER 5").message,
            "no viable alternative at input 'GREATER5'"
        );
        assert_eq!(
            parse_err("[age] NOT 5").message,
            "no viable alternative at input '[age]NOT5'"
        );
    }

    #[test]
    fn unbalanced_parens_are_rejected() {
        assert_eq!(parse_err("([age] > 1").message, "missing ')' at '<EOF>'");
        assert_eq!(
            parse_err("([age] > 1 AND [age] < 2").message,
            "missing ')' at '<EOF>'"
        );
        // A token that cannot follow the group is a mismatch, not an omission.
        assert_eq!(
            parse_err("([age] > 1 5").message,
            "mismatched input '5' expecting ')'"
        );
        assert_eq!(
            parse_err("[age] > 1)").message,
            "extraneous input ')' expecting <EOF>"
        );
    }

    #[test]
    fn a_missing_token_is_reported_as_missing_when_one_would_do() {
        // Insertion fires only where the offending token could legally follow
        // the one left out.
        for (query, expected) in [
            ("[age] is not", "missing BLANK at '<EOF>'"),
            ("[age] is not AND [age] > 1", "missing BLANK at 'AND'"),
            ("[name] contains", "missing STRING at '<EOF>'"),
            ("[name] not contains", "missing STRING at '<EOF>'"),
            ("[name] starts with", "missing STRING at '<EOF>'"),
            ("[name] contains AND [age] > 1", "missing STRING at 'AND'"),
            ("[age] greater than or 5", "missing EQUAL at '5'"),
            ("[age] = -", "missing DIGITS at '<EOF>'"),
            ("[age] = 5.", "missing DIGITS at '<EOF>'"),
            ("[age] = 5. AND [age] > 1", "missing DIGITS at 'AND'"),
            ("([age] = 5.)", "missing DIGITS at ')'"),
        ] {
            assert_eq!(parse_err(query).message, expected, "{query}");
        }
    }

    #[test]
    fn a_deletable_token_is_reported_as_extraneous() {
        // Deletion is tried before insertion, so the doubled sign is blamed
        // rather than a digit being called missing.
        for (query, expected) in [
            ("[age] = --5", "extraneous input '-' expecting DIGITS"),
            ("[age] = -.5", "extraneous input '.' expecting DIGITS"),
            (
                "[age] = .5",
                "extraneous input '.' expecting {STRING, TRUE, FALSE, SIGN, DIGITS}",
            ),
            (
                "[age] > = 5",
                "extraneous input '=' expecting {STRING, TRUE, FALSE, SIGN, DIGITS}",
            ),
            (
                "AND [age] > 1",
                "extraneous input 'AND' expecting {FIELD, NOT, '('}",
            ),
            ("[age] > 1 1", "extraneous input '1' expecting <EOF>"),
            (
                "[age] is blank blank",
                "extraneous input 'blank' expecting <EOF>",
            ),
        ] {
            assert_eq!(parse_err(query).message, expected, "{query}");
        }
    }

    #[test]
    fn an_undeletable_unomittable_token_is_a_mismatch() {
        for (query, expected) in [
            ("[name] starts 5", "mismatched input '5' expecting WITH"),
            ("[name] ends 5", "mismatched input '5' expecting WITH"),
            ("[name] starts", "mismatched input '<EOF>' expecting WITH"),
            ("[name] ends", "mismatched input '<EOF>' expecting WITH"),
            (
                "[name] starts AND [age] > 1",
                "mismatched input 'AND' expecting WITH",
            ),
            ("[name] is not 5", "mismatched input '5' expecting BLANK"),
            ("[name] contains 5", "mismatched input '5' expecting STRING"),
            (
                "[name] contains true",
                "mismatched input 'true' expecting STRING",
            ),
            ("[age] = 5.true", "mismatched input 'true' expecting DIGITS"),
            (
                "[age] greater than or",
                "mismatched input '<EOF>' expecting EQUAL",
            ),
            ("[age] = 5 = 5", "mismatched input '=' expecting <EOF>"),
            ("[age] > 1 1 1", "mismatched input '1' expecting <EOF>"),
        ] {
            assert_eq!(parse_err(query).message, expected, "{query}");
        }
    }

    // --- number handling ---------------------------------------------------

    #[test]
    fn signs_are_applied() {
        for query in ["[age] = -5", "[age] = - 5"] {
            assert_eq!(
                parse(query).filters[0].condition.as_ref().unwrap().args[0],
                json!(-5),
                "{query}"
            );
        }
        for query in ["[age] = +5", "[age] = + 5"] {
            assert_eq!(
                parse(query).filters[0].condition.as_ref().unwrap().args[0],
                json!(5),
                "{query}"
            );
        }
    }

    #[test]
    fn trailing_fraction_zeros_are_normalized() {
        assert_eq!(
            parse("[age] = 1.500").filters[0]
                .condition
                .as_ref()
                .unwrap()
                .args[0],
            json!(1.5)
        );
        // `1.000` is integral, and the reference emits `1` for it — JavaScript
        // has one number type, so a written fraction that happens to be whole
        // still serializes without a decimal point.
        let group = parse("[age] = 1.000");
        let one = &group.filters[0].condition.as_ref().unwrap().args[0];
        assert_eq!(one, &json!(1));
        assert_eq!(one.to_string(), "1");
    }

    #[test]
    fn an_unrepresentable_number_becomes_null() {
        // `parseFloat` overflows to Infinity in the reference, which serializes
        // as `null`; the same arg must appear here.
        let query = format!("[age] = {}", "9".repeat(400));
        assert_eq!(
            parse(&query).filters[0].condition.as_ref().unwrap().args[0],
            serde_json::Value::Null
        );
    }

    // --- parse_query and ParseResult ---------------------------------------

    #[test]
    fn parse_result_is_filters_or_errors_never_both() {
        let cols = vec![ColumnDef::new("age", crate::column::ColumnType::Number)];
        let ok = parse_query("[age] > 1", &cols);
        assert!(ok.filters.is_some());
        assert!(ok.errors.is_none());
        assert!(!ok.has_errors());
        assert!(ok.errors().is_empty());

        let bad = parse_query("[nope] > 1", &cols);
        assert!(bad.filters.is_none());
        assert!(bad.has_errors());
        assert_eq!(bad.errors().len(), 1);
    }

    #[test]
    fn parse_result_json_omits_the_absent_side() {
        let ok = ParseResult::ok(FilterGroup::default());
        let value = serde_json::to_value(&ok).unwrap();
        assert!(value.get("errors").is_none());
        assert!(value.get("filters").is_some());

        let bad = ParseResult::failed(vec![ParseError::new("x", 0, 0)]);
        let value = serde_json::to_value(&bad).unwrap();
        assert!(value.get("filters").is_none());
        assert_eq!(value["errors"][0]["severity"], json!("error"));
    }

    #[test]
    fn syntax_errors_preempt_semantic_ones() {
        // `[nope]` does not exist *and* the trailing `1` is extraneous; the
        // syntax error is the one reported.
        let cols = vec![ColumnDef::new("age", crate::column::ColumnType::Number)];
        let result = parse_query("[nope] > 1 1", &cols);
        assert_eq!(
            result.errors()[0].message,
            "extraneous input '1' expecting <EOF>"
        );
    }

    #[test]
    fn error_spans_are_byte_offsets() {
        // The documented divergence, pinned as a difference rather than an
        // agreement: `é` is two bytes but one UTF-16 code unit, so the reference
        // reports 11 for this input where the byte offset is 12. The span must
        // still slice the input correctly, which is what makes it the useful one.
        let query = "[café] > 1 1";
        let e = parse_err(query);
        assert_eq!((e.start, e.end), (12, 13));
        assert_eq!(&query[e.start..e.end], "1");
        // The same query in pure ASCII agrees with the reference exactly, and the
        // gap between the two is precisely the extra byte `é` costs.
        let ascii = parse_err("[cafe] > 1 1");
        assert_eq!((ascii.start, ascii.end), (11, 12));
        assert_eq!(e.start - ascii.start, query.len() - "[cafe] > 1 1".len());
    }

    #[test]
    fn severity_serializes_lowercase() {
        assert_eq!(
            serde_json::to_value(ErrorSeverity::Error).unwrap(),
            json!("error")
        );
        assert_eq!(
            serde_json::to_value(ErrorSeverity::Warning).unwrap(),
            json!("warning")
        );
        assert_eq!(ErrorSeverity::default(), ErrorSeverity::Error);
    }
}
