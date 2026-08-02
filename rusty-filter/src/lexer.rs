//! Hand-written lexer for the `Filters.g4` token set.
//!
//! One [`Token`] variant per lexer rule of the grammar. Every rule below was
//! checked against the shipped `filter-query-editor` 2.2.0 bundle, both through
//! `parseQuery` and by driving the generated ANTLR lexer directly; see the crate
//! docs for the probe method.
//!
//! Whitespace (space, tab, CR, LF) is skipped between tokens, which is why
//! `1 . 5` lexes as three tokens and parses as `1.5`.
//!
//! # How lexer errors are shaped
//!
//! ANTLR's lexer walks its DFA as far as the input stays a *viable prefix* of
//! some rule, and its error message quotes everything it walked over plus the
//! character that broke viability. That is why `1e5` complains about `e5` rather
//! than `e`, and why `[age] ! 5` complains about `'! '` — bang and space — since
//! `!` is a viable prefix of `!=` and the space is what rules it out.
//! [`viable_len`] is that walk, and it is the reason the error text is built the
//! way it is instead of just naming the offending character.

/// A lexed token together with its byte span in the input.
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    /// The raw lexeme, exactly as it appeared in the input: brackets included
    /// for a field, quotes included for a string, original case for a keyword.
    /// Empty for [`Token::Eof`]. Error messages quote this rather than a
    /// canonical spelling, so `[age] GREATER 5` complains about `GREATER`.
    pub text: String,
    /// Byte offset of the first character of the token.
    pub start: usize,
    /// Byte offset one past the last character of the token.
    pub end: usize,
}

impl SpannedToken {
    /// The spelling an error message uses for this token.
    pub fn describe(&self) -> &str {
        if self.token == Token::Eof {
            "<EOF>"
        } else {
            &self.text
        }
    }
}

/// The terminals of the grammar.
///
/// [`Token::Field`] carries the text *between* the brackets, verbatim: no
/// trimming and no unescaping. [`Token::Str`] carries the *unescaped* body of
/// the string literal. Both keep their raw lexeme in [`SpannedToken::text`].
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Field(String),
    Str(String),
    Contains,
    Greater,
    Starts,
    Equals,
    Equal,
    Blank,
    Less,
    Than,
    Ends,
    With,
    Not,
    Or,
    Is,
    And,
    True,
    False,
    LParen,
    RParen,
    Eq,
    Eq2,
    Neq,
    Gt,
    Ge,
    Lt,
    Le,
    Dot,
    Sign(char),
    Digits(String),
    Eof,
}

impl Token {
    /// The grammar's name for this token, used where a message names an
    /// *expected* token rather than quoting the offending one — as in
    /// `missing BLANK at '<EOF>'`.
    pub fn name(&self) -> &'static str {
        match self {
            Token::Field(_) => "FIELD",
            Token::Str(_) => "STRING",
            Token::Contains => "CONTAINS",
            Token::Greater => "GREATER",
            Token::Starts => "STARTS",
            Token::Equals => "EQUALS",
            Token::Equal => "EQUAL",
            Token::Blank => "BLANK",
            Token::Less => "LESS",
            Token::Than => "THAN",
            Token::Ends => "ENDS",
            Token::With => "WITH",
            Token::Not => "NOT",
            Token::Or => "OR",
            Token::Is => "IS",
            Token::And => "AND",
            Token::True => "TRUE",
            Token::False => "FALSE",
            Token::LParen => "'('",
            Token::RParen => "')'",
            Token::Eq => "EQUAL_SIGN",
            Token::Eq2 => "EQUAL_SIGN2",
            Token::Neq => "NOT_EQUAL_SIGN",
            Token::Gt => "GT",
            Token::Ge => "GTE",
            Token::Lt => "LT",
            Token::Le => "LTE",
            Token::Dot => "DOT",
            Token::Sign(_) => "SIGN",
            Token::Digits(_) => "DIGITS",
            Token::Eof => "<EOF>",
        }
    }
}

/// A lexing failure, with the byte span of the offending text.
#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub message: String,
    pub start: usize,
    pub end: usize,
}

/// The keyword table. Matching is ASCII-case-insensitive, and the table is also
/// what [`viable_len`] consults to decide whether a partial word could still
/// grow into a keyword.
const KEYWORDS: &[(&str, Token)] = &[
    ("contains", Token::Contains),
    ("greater", Token::Greater),
    ("starts", Token::Starts),
    ("equals", Token::Equals),
    ("equal", Token::Equal),
    ("blank", Token::Blank),
    ("false", Token::False),
    ("less", Token::Less),
    ("than", Token::Than),
    ("ends", Token::Ends),
    ("with", Token::With),
    ("true", Token::True),
    ("not", Token::Not),
    ("and", Token::And),
    ("or", Token::Or),
    ("is", Token::Is),
];

/// The multi-character symbolic tokens, longest first.
const SYMBOLS2: &[(&str, Token)] = &[
    ("==", Token::Eq2),
    ("!=", Token::Neq),
    (">=", Token::Ge),
    ("<=", Token::Le),
];

/// Lex `input` into tokens, always terminated by [`Token::Eof`].
///
/// Returns the first error encountered rather than recovering; see the crate
/// docs on the single-error divergence from the ANTLR reference.
pub fn tokenize(input: &str) -> Result<Vec<SpannedToken>, LexError> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        let c = bytes[i];
        // WS: space, tab, CR, LF — skipped, not emitted.
        if matches!(c, b' ' | b'\t' | b'\r' | b'\n') {
            i += 1;
            continue;
        }
        match longest_match(input, i) {
            Some((token, len)) => {
                tokens.push(SpannedToken {
                    token,
                    text: input[i..i + len].to_string(),
                    start: i,
                    end: i + len,
                });
                i += len;
            }
            None => return Err(recognition_error(input, i)),
        }
    }

    tokens.push(SpannedToken {
        token: Token::Eof,
        text: String::new(),
        start: input.len(),
        end: input.len(),
    });
    Ok(tokens)
}

/// The longest complete token starting at byte offset `at`, with its length.
///
/// Maximal munch: `==` beats `=`, `equals` beats `equal`, and a digit run is
/// taken whole. A word that is only a *prefix* of a keyword matches nothing,
/// which is what makes `end` an error while `ends` is a token.
fn longest_match(input: &str, at: usize) -> Option<(Token, usize)> {
    let bytes = input.as_bytes();
    let rest = &input[at..];
    let c = bytes[at];

    // FIELD: '[' ~[\r\n\]]+ ']'
    if c == b'[' {
        let mut j = at + 1;
        while j < bytes.len() {
            match bytes[j] {
                b'\r' | b'\n' => return None,
                // The `+` needs at least one inner character, so `[]` is not a
                // field.
                b']' if j > at + 1 => {
                    return Some((Token::Field(input[at + 1..j].to_string()), j + 1 - at));
                }
                b']' => return None,
                _ => j += 1,
            }
        }
        return None;
    }

    // STRING: '"' ( '\\' . | ~[\\"\r\n] )* '"'
    if c == b'"' {
        let mut j = at + 1;
        let mut body = String::new();
        while j < bytes.len() {
            match bytes[j] {
                // An escape consumes the next character whatever it is, so a
                // trailing backslash swallows the closing quote.
                b'\\' => match char_at(input, j + 1) {
                    Some((ch, width)) => {
                        body.push('\\');
                        body.push(ch);
                        j += 1 + width;
                    }
                    None => return None,
                },
                b'"' => return Some((Token::Str(unescape(&body)), j + 1 - at)),
                b'\r' | b'\n' => return None,
                _ => {
                    let (ch, width) = char_at(input, j).expect("index inside input");
                    body.push(ch);
                    j += width;
                }
            }
        }
        return None;
    }

    // Two-character operators before their one-character prefixes.
    for (text, token) in SYMBOLS2 {
        if rest.starts_with(text) {
            return Some((token.clone(), text.len()));
        }
    }
    let single = match c {
        b'(' => Some(Token::LParen),
        b')' => Some(Token::RParen),
        b'=' => Some(Token::Eq),
        b'>' => Some(Token::Gt),
        b'<' => Some(Token::Lt),
        b'.' => Some(Token::Dot),
        b'+' => Some(Token::Sign('+')),
        b'-' => Some(Token::Sign('-')),
        _ => None,
    };
    if let Some(token) = single {
        return Some((token, 1));
    }

    // DIGITS: [0-9]+
    if c.is_ascii_digit() {
        let len = bytes[at..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        return Some((Token::Digits(input[at..at + len].to_string()), len));
    }

    // Keywords, longest complete spelling wins.
    let mut best: Option<(Token, usize)> = None;
    for (word, token) in KEYWORDS {
        if rest.len() >= word.len() && rest[..word.len()].eq_ignore_ascii_case(word) {
            let better = best.as_ref().is_none_or(|(_, len)| word.len() > *len);
            if better {
                best = Some((token.clone(), word.len()));
            }
        }
    }
    best
}

/// How many bytes from `at` stay a viable prefix of some lexer rule.
///
/// Zero for a character that starts no rule at all. This is the DFA walk ANTLR
/// performs before it gives up, and the length its error message is built from.
fn viable_len(input: &str, at: usize) -> usize {
    let bytes = input.as_bytes();
    let rest = &input[at..];
    let c = bytes[at];

    // A field or string stays viable until a forbidden character appears; the
    // whole remainder is viable if it simply runs out unterminated.
    if c == b'[' {
        let mut j = at + 1;
        while j < bytes.len() && !matches!(bytes[j], b'\r' | b'\n' | b']') {
            j += 1;
        }
        return j - at;
    }
    if c == b'"' {
        let mut j = at + 1;
        while j < bytes.len() {
            match bytes[j] {
                b'\\' => match char_at(input, j + 1) {
                    Some((_, width)) => j += 1 + width,
                    None => {
                        j += 1;
                        break;
                    }
                },
                b'\r' | b'\n' => break,
                _ => j += char_at(input, j).expect("index inside input").1,
            }
        }
        return j - at;
    }

    // `!` alone is viable only because `!=` exists.
    if c == b'!' {
        return 1;
    }
    // Every other symbolic token and a digit run are complete as soon as they
    // start, so nothing beyond `longest_match` is viable.
    if let Some((_, len)) = longest_match(input, at) {
        if !c.is_ascii_alphabetic() {
            return len;
        }
    }

    // A word is viable while it is a case-insensitive prefix of some keyword.
    let mut best = 0;
    for (word, _) in KEYWORDS {
        let mut shared = 0;
        while shared < word.len()
            && shared < rest.len()
            && rest.as_bytes()[shared].eq_ignore_ascii_case(&word.as_bytes()[shared])
        {
            shared += 1;
        }
        best = best.max(shared);
    }
    best
}

/// Build the `token recognition error` the reference reports at `at`.
///
/// The quoted text runs from `at` over every viable byte plus the character that
/// broke viability, or to the end of the input if viability ran out there. The
/// *span*, by contrast, covers only the first character: the reference's error
/// listener gets no token for a lexer error and falls back to a one-unit span at
/// the reported position.
fn recognition_error(input: &str, at: usize) -> LexError {
    let viable = viable_len(input, at);
    let end = match char_at(input, at + viable) {
        Some((_, width)) => at + viable + width,
        None => input.len(),
    };
    let (_, first_width) = char_at(input, at).expect("index inside input");
    LexError {
        message: format!("token recognition error at: '{}'", &input[at..end]),
        start: at,
        end: at + first_width,
    }
}

/// The character starting at byte offset `at`, with its UTF-8 width.
fn char_at(input: &str, at: usize) -> Option<(char, usize)> {
    input
        .get(at..)
        .and_then(|s| s.chars().next())
        .map(|c| (c, c.len_utf8()))
}

/// Unescape a string literal body exactly as `extractStringValue` does.
///
/// The reference applies three ordered replacements to the quoted body:
/// `\"` then `\'` then `\\`. Only those three escapes are recognised; every
/// other escape **keeps its backslash**, so `\t` stays as backslash-t and
/// `\z` stays as backslash-z. Because the replacements run left to right over
/// the whole string rather than as a single scan, a `\\` pair is only collapsed
/// after the quote escapes have been rewritten — which is why `\\'` yields
/// backslash-quote rather than backslash-backslash-quote.
fn unescape(body: &str) -> String {
    let stage1 = body.replace("\\\"", "\"");
    let stage2 = stage1.replace("\\'", "'");
    stage2.replace("\\\\", "\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(input: &str) -> Vec<Token> {
        tokenize(input)
            .expect("expected the input to lex")
            .into_iter()
            .map(|t| t.token)
            .collect()
    }

    fn err(input: &str) -> LexError {
        tokenize(input).expect_err("expected the input to fail lexing")
    }

    #[test]
    fn lexes_every_token_variant() {
        let tokens = toks(
            "[f] \"s\" contains greater starts equals equal blank less than ends with \
             not or is and true false ( ) = == != > >= < <= . + - 12",
        );
        assert_eq!(
            tokens,
            vec![
                Token::Field("f".to_string()),
                Token::Str("s".to_string()),
                Token::Contains,
                Token::Greater,
                Token::Starts,
                Token::Equals,
                Token::Equal,
                Token::Blank,
                Token::Less,
                Token::Than,
                Token::Ends,
                Token::With,
                Token::Not,
                Token::Or,
                Token::Is,
                Token::And,
                Token::True,
                Token::False,
                Token::LParen,
                Token::RParen,
                Token::Eq,
                Token::Eq2,
                Token::Neq,
                Token::Gt,
                Token::Ge,
                Token::Lt,
                Token::Le,
                Token::Dot,
                Token::Sign('+'),
                Token::Sign('-'),
                Token::Digits("12".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn field_keeps_inner_text_verbatim() {
        assert_eq!(toks("[a b]")[0], Token::Field("a b".to_string()));
        assert_eq!(toks("[a[b]")[0], Token::Field("a[b".to_string()));
        assert_eq!(toks("[café]")[0], Token::Field("café".to_string()));
        assert_eq!(toks("[a\tb]")[0], Token::Field("a\tb".to_string()));
        // No trimming: a single space is a legal one-character column name.
        assert_eq!(toks("[ ]")[0], Token::Field(" ".to_string()));
        assert_eq!(toks("[ s ]")[0], Token::Field(" s ".to_string()));
    }

    #[test]
    fn raw_text_is_kept_alongside_the_token() {
        let tokens = tokenize("[age] GREATER \"x\"").unwrap();
        assert_eq!(tokens[0].text, "[age]");
        // Original case, so an error message can quote what was typed.
        assert_eq!(tokens[1].text, "GREATER");
        assert_eq!(tokens[2].text, "\"x\"");
        assert_eq!(tokens[3].describe(), "<EOF>");
        assert_eq!(tokens[3].text, "");
    }

    #[test]
    fn empty_field_is_rejected() {
        // Viability stops after `[`, so the `]` that broke it is quoted too.
        let e = err("[] = \"x\"");
        assert_eq!(e.message, "token recognition error at: '[]'");
        assert_eq!((e.start, e.end), (0, 1));
    }

    #[test]
    fn field_with_escaped_bracket_is_rejected() {
        // `FIELD` has no escape rule, so the backslash-bracket closes the field
        // and the remainder fails, exactly as in the bundle.
        let tokens = tokenize("[a\\]").unwrap();
        assert_eq!(tokens[0].token, Token::Field("a\\".to_string()));
        let e = err("[a\\]b] = \"x\"");
        assert_eq!(e.message, "token recognition error at: 'b]'");
        assert_eq!((e.start, e.end), (4, 5));
    }

    #[test]
    fn field_with_raw_newline_is_rejected() {
        let e = err("[a\nb] = \"x\"");
        assert_eq!(e.message, "token recognition error at: '[a\n'");
        assert_eq!((e.start, e.end), (0, 1));
        let e = err("[a\rb] = \"x\"");
        assert_eq!(e.message, "token recognition error at: '[a\r'");
    }

    #[test]
    fn unterminated_field_quotes_the_whole_remainder() {
        let e = err("[age");
        assert_eq!(e.message, "token recognition error at: '[age'");
        assert_eq!((e.start, e.end), (0, 1));
    }

    #[test]
    fn string_escapes_follow_the_reference() {
        // Only the three recognised escapes are rewritten.
        assert_eq!(toks("\"a\\\"b\"")[0], Token::Str("a\"b".to_string()));
        assert_eq!(toks("\"a\\'b\"")[0], Token::Str("a'b".to_string()));
        assert_eq!(toks("\"a\\\\b\"")[0], Token::Str("a\\b".to_string()));
        // Everything else keeps its backslash.
        assert_eq!(toks("\"a\\tb\"")[0], Token::Str("a\\tb".to_string()));
        assert_eq!(toks("\"a\\nb\"")[0], Token::Str("a\\nb".to_string()));
        assert_eq!(toks("\"a\\zb\"")[0], Token::Str("a\\zb".to_string()));
    }

    #[test]
    fn string_escape_replacement_order_is_sequential() {
        // `\\'` collapses to backslash-quote because the `\'` rewrite runs
        // before the `\\` one; a single scan would give backslash-backslash-quote.
        assert_eq!(toks("\"\\\\'\"")[0], Token::Str("\\'".to_string()));
        assert_eq!(toks("\"\\\\\\\\'\"")[0], Token::Str("\\\\'".to_string()));
        assert_eq!(toks("\"\\\\t\"")[0], Token::Str("\\t".to_string()));
    }

    #[test]
    fn raw_newline_in_string_is_rejected() {
        let e = err("[s] = \"a\nb\"");
        assert_eq!(e.message, "token recognition error at: '\"a\n'");
        assert_eq!((e.start, e.end), (6, 7));
    }

    #[test]
    fn raw_tab_in_string_is_accepted() {
        assert_eq!(toks("\"a\tb\"")[0], Token::Str("a\tb".to_string()));
    }

    #[test]
    fn unterminated_string_is_rejected() {
        let e = err("\"abc");
        assert_eq!(e.message, "token recognition error at: '\"abc'");
        // A trailing backslash consumes the closing quote.
        let e = err("[s] = \"a\\\"");
        assert_eq!(e.message, "token recognition error at: '\"a\\\"'");
        assert_eq!((e.start, e.end), (6, 7));
    }

    #[test]
    fn single_quoted_string_is_rejected() {
        let e = err("[name] = 'x'");
        assert_eq!(e.message, "token recognition error at: '''");
        assert_eq!((e.start, e.end), (9, 10));
    }

    #[test]
    fn two_character_operators_win_maximal_munch() {
        assert_eq!(toks("==")[0], Token::Eq2);
        assert_eq!(toks("!=")[0], Token::Neq);
        assert_eq!(toks(">=")[0], Token::Ge);
        assert_eq!(toks("<=")[0], Token::Le);
    }

    #[test]
    fn spaced_two_character_operator_lexes_as_two_tokens() {
        assert_eq!(toks("> ="), vec![Token::Gt, Token::Eq, Token::Eof]);
        assert_eq!(toks("< ="), vec![Token::Lt, Token::Eq, Token::Eof]);
        assert_eq!(toks("= ="), vec![Token::Eq, Token::Eq, Token::Eof]);
    }

    #[test]
    fn lone_bang_is_rejected_and_quotes_the_next_character() {
        // `!` is a viable prefix of `!=`, so the space that rules it out is part
        // of the quoted text.
        let e = err("[age] ! 5");
        assert_eq!(e.message, "token recognition error at: '! '");
        assert_eq!((e.start, e.end), (6, 7));
        assert_eq!(err("[age] !x").message, "token recognition error at: '!x'");
        assert_eq!(err("!").message, "token recognition error at: '!'");
    }

    #[test]
    fn exponent_syntax_is_rejected() {
        // `e` could still become `equal`, `equals` or `ends`, so the `5` that
        // rules all three out is quoted with it.
        let e = err("[age] = 1e5");
        assert_eq!(e.message, "token recognition error at: 'e5'");
        assert_eq!((e.start, e.end), (9, 10));
        assert_eq!(
            err("[age] = 1E5").message,
            "token recognition error at: 'E5'"
        );
        assert_eq!(
            err("[age] = 1ee5").message,
            "token recognition error at: 'ee'"
        );
        assert_eq!(err("[age] = 1e").message, "token recognition error at: 'e'");
    }

    #[test]
    fn a_keyword_followed_by_letters_lexes_then_fails() {
        // Maximal munch takes the keyword, then the leftover letters fail — so
        // the input as a whole does not lex and the error names only the tail.
        // `tokenize` reports that first failure instead of the tokens preceding
        // it, which is why this asserts through `err` and not `toks`.
        let e = err("andx");
        assert_eq!(e.message, "token recognition error at: 'x'");
        assert_eq!((e.start, e.end), (3, 4));
        assert_eq!(err("notx").message, "token recognition error at: 'x'");
        assert_eq!(err("equalx").message, "token recognition error at: 'x'");
        assert_eq!(err("equalsx").message, "token recognition error at: 'x'");
        assert_eq!(err("blanks").message, "token recognition error at: 's'");
        // `a` is viable as a prefix of `and`, so the space is quoted with it.
        assert_eq!(err("nota ").message, "token recognition error at: 'a '");
    }

    #[test]
    fn adjacent_keywords_lex_without_separators() {
        assert_eq!(toks("andor"), vec![Token::And, Token::Or, Token::Eof]);
        assert_eq!(toks("isnot"), vec![Token::Is, Token::Not, Token::Eof]);
        assert_eq!(
            toks("isnotblank"),
            vec![Token::Is, Token::Not, Token::Blank, Token::Eof]
        );
        assert_eq!(
            toks("lessthanorequal"),
            vec![
                Token::Less,
                Token::Than,
                Token::Or,
                Token::Equal,
                Token::Eof
            ]
        );
        assert_eq!(
            toks("startswith"),
            vec![Token::Starts, Token::With, Token::Eof]
        );
    }

    #[test]
    fn a_keyword_prefix_alone_is_an_error() {
        // Measured: `ends` lexes, `end`, `en` and `e` do not.
        assert_eq!(toks("ends"), vec![Token::Ends, Token::Eof]);
        assert_eq!(err("end").message, "token recognition error at: 'end'");
        assert_eq!(err("en").message, "token recognition error at: 'en'");
        assert_eq!(err("e").message, "token recognition error at: 'e'");
    }

    #[test]
    fn an_unviable_character_quotes_only_itself() {
        for (input, text) in [
            ("z", "z"),
            ("q", "q"),
            ("@", "@"),
            ("#", "#"),
            ("~", "~"),
            ("zz", "z"),
        ] {
            assert_eq!(
                err(input).message,
                format!("token recognition error at: '{text}'"),
                "{input}"
            );
        }
        // But a viable prefix drags in the character that broke it.
        assert_eq!(err("ab").message, "token recognition error at: 'ab'");
        assert_eq!(err("abc").message, "token recognition error at: 'ab'");
        assert_eq!(err("an ").message, "token recognition error at: 'an '");
        assert_eq!(err("sta ").message, "token recognition error at: 'sta '");
        assert_eq!(err("i ").message, "token recognition error at: 'i '");
    }

    #[test]
    fn keywords_are_case_insensitive() {
        for spelling in ["AND", "and", "AnD"] {
            assert_eq!(toks(spelling)[0], Token::And);
        }
        for spelling in ["CONTAINS", "contains", "CoNtAiNs"] {
            assert_eq!(toks(spelling)[0], Token::Contains);
        }
        assert_eq!(toks("TRUE")[0], Token::True);
        assert_eq!(toks("False")[0], Token::False);
    }

    #[test]
    fn equals_wins_over_equal() {
        assert_eq!(toks("equals"), vec![Token::Equals, Token::Eof]);
        assert_eq!(toks("equal"), vec![Token::Equal, Token::Eof]);
    }

    #[test]
    fn digits_are_grouped_and_dot_is_separate() {
        assert_eq!(
            toks("1 . 5"),
            vec![
                Token::Digits("1".to_string()),
                Token::Dot,
                Token::Digits("5".to_string()),
                Token::Eof,
            ]
        );
        assert_eq!(toks("007")[0], Token::Digits("007".to_string()));
    }

    #[test]
    fn keywords_need_no_surrounding_space() {
        assert_eq!(
            toks("[age]>1AND[age]<5"),
            vec![
                Token::Field("age".to_string()),
                Token::Gt,
                Token::Digits("1".to_string()),
                Token::And,
                Token::Field("age".to_string()),
                Token::Lt,
                Token::Digits("5".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn whitespace_is_skipped() {
        assert_eq!(toks("  \t\r\n  "), vec![Token::Eof]);
        assert_eq!(toks(""), vec![Token::Eof]);
    }

    #[test]
    fn spans_are_byte_offsets() {
        let tokens = tokenize("[café] = \"x\"").unwrap();
        assert_eq!((tokens[0].start, tokens[0].end), (0, 7));
        assert_eq!((tokens[1].start, tokens[1].end), (8, 9));
        assert_eq!((tokens[2].start, tokens[2].end), (10, 13));
        assert_eq!(tokens[3].token, Token::Eof);
        assert_eq!(tokens[3].start, "[café] = \"x\"".len());
    }

    #[test]
    fn a_multibyte_offending_character_gets_a_whole_char_span() {
        // The reference would report a one-code-unit span here; a byte-offset
        // port reports the character's full width so the span stays sliceable.
        let e = err("é");
        assert_eq!(e.message, "token recognition error at: 'é'");
        assert_eq!((e.start, e.end), (0, 2));
        assert_eq!(&"é"[e.start..e.end], "é");
    }

    #[test]
    fn token_names_are_the_grammars() {
        assert_eq!(Token::Blank.name(), "BLANK");
        assert_eq!(Token::With.name(), "WITH");
        assert_eq!(Token::RParen.name(), "')'");
        assert_eq!(Token::Digits("1".to_string()).name(), "DIGITS");
        assert_eq!(Token::Eof.name(), "<EOF>");
    }

    #[test]
    fn viability_is_zero_for_a_character_that_starts_nothing() {
        assert_eq!(viable_len("z", 0), 0);
        assert_eq!(viable_len("!", 0), 1);
        assert_eq!(viable_len("e", 0), 1);
        assert_eq!(viable_len("end", 0), 3);
        assert_eq!(viable_len("ends", 0), 4);
        assert_eq!(viable_len("[ab", 0), 3);
        assert_eq!(viable_len("=", 0), 1);
        assert_eq!(viable_len("12", 0), 2);
    }
}
