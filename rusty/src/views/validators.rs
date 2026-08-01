//! Field validation predicates for use with [`crate::views::form_builder::FormBuilder`].
//!
//! Each validator takes the field's string value and returns `Ok(())` when valid,
//! or `Err(message)` with a human-readable error. Checks are hand-rolled — the
//! crate deliberately depends on no regex or URL parsing library.
//!
//! Following Ivy's `Validators`, the format checks (`email`, `url`) treat an empty
//! or whitespace-only value as valid; use [`not_empty`] to require a value.

/// Reject an empty or whitespace-only value.
pub fn not_empty(value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err("This field is required".to_string())
    } else {
        Ok(())
    }
}

/// Require at least `n` characters (counted as Unicode scalar values).
pub fn min_length(value: &str, n: usize) -> Result<(), String> {
    if value.chars().count() < n {
        Err(format!("Must be at least {} characters", n))
    } else {
        Ok(())
    }
}

/// Require at most `n` characters (counted as Unicode scalar values).
pub fn max_length(value: &str, n: usize) -> Result<(), String> {
    if value.chars().count() > n {
        Err(format!("Must be at most {} characters", n))
    } else {
        Ok(())
    }
}

/// Accept a single `local@host` address whose host contains a dot.
/// An empty value passes — pair with [`not_empty`] when the field is required.
pub fn email(value: &str) -> Result<(), String> {
    const ERR: &str = "Please enter a valid email address";
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Err(ERR.to_string());
    }

    let mut parts = trimmed.split('@');
    let local = parts.next().unwrap_or("");
    let host = parts.next().unwrap_or("");
    if parts.next().is_some() || local.is_empty() || host.is_empty() {
        return Err(ERR.to_string());
    }
    // The host must have a dotted label with non-empty parts on both sides.
    if !host.contains('.') || host.starts_with('.') || host.ends_with('.') || host.contains("..") {
        return Err(ERR.to_string());
    }
    Ok(())
}

/// Accept an absolute `http`/`https` URL with a non-empty host.
/// An empty value passes — pair with [`not_empty`] when the field is required.
pub fn url(value: &str) -> Result<(), String> {
    const ERR: &str = "Please enter a valid URL";
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Err(ERR.to_string());
    }

    let rest = match trimmed.split_once("://") {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("http") => rest,
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("https") => rest,
        _ => return Err(ERR.to_string()),
    };

    // Strip path, query and fragment to isolate the authority.
    let host = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .rsplit('@')
        .next()
        .unwrap_or("");
    if host.is_empty() {
        return Err(ERR.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_empty() {
        let cases: &[(&str, bool)] = &[
            ("Alice", true),
            ("  x  ", true),
            ("", false),
            ("   ", false),
            ("\t\n", false),
        ];
        for (input, expected) in cases {
            assert_eq!(
                not_empty(input).is_ok(),
                *expected,
                "not_empty({:?}) mismatch",
                input
            );
        }
    }

    #[test]
    fn test_min_length() {
        let cases: &[(&str, usize, bool)] = &[
            ("abc", 3, true),
            ("abcd", 3, true),
            ("ab", 3, false),
            ("", 1, false),
            ("", 0, true),
            ("äöü", 3, true),
        ];
        for (input, n, expected) in cases {
            assert_eq!(
                min_length(input, *n).is_ok(),
                *expected,
                "min_length({:?}, {}) mismatch",
                input,
                n
            );
        }
    }

    #[test]
    fn test_max_length() {
        let cases: &[(&str, usize, bool)] = &[
            ("abc", 3, true),
            ("ab", 3, true),
            ("abcd", 3, false),
            ("", 0, true),
            ("äöü", 3, true),
            ("äöüx", 3, false),
        ];
        for (input, n, expected) in cases {
            assert_eq!(
                max_length(input, *n).is_ok(),
                *expected,
                "max_length({:?}, {}) mismatch",
                input,
                n
            );
        }
    }

    #[test]
    fn test_email() {
        let cases: &[(&str, bool)] = &[
            ("user@example.com", true),
            ("first.last@sub.example.co.uk", true),
            ("  user@example.com  ", true),
            ("", true),
            ("   ", true),
            ("user@localhost", false),
            ("user", false),
            ("@example.com", false),
            ("user@", false),
            ("a@b@example.com", false),
            ("user@.com", false),
            ("user@example.", false),
            ("user@exa..mple.com", false),
            ("us er@example.com", false),
        ];
        for (input, expected) in cases {
            assert_eq!(
                email(input).is_ok(),
                *expected,
                "email({:?}) mismatch",
                input
            );
        }
    }

    #[test]
    fn test_url() {
        let cases: &[(&str, bool)] = &[
            ("https://example.com", true),
            ("http://example.com/path?q=1#frag", true),
            ("HTTPS://EXAMPLE.COM", true),
            ("https://user:pw@example.com", true),
            ("  https://example.com  ", true),
            ("", true),
            ("   ", true),
            ("example.com", false),
            ("ftp://example.com", false),
            ("https://", false),
            ("https:///path", false),
            ("https://exa mple.com", false),
        ];
        for (input, expected) in cases {
            assert_eq!(url(input).is_ok(), *expected, "url({:?}) mismatch", input);
        }
    }

    #[test]
    fn test_error_messages() {
        assert_eq!(not_empty("").unwrap_err(), "This field is required");
        assert_eq!(
            min_length("a", 4).unwrap_err(),
            "Must be at least 4 characters"
        );
        assert_eq!(
            max_length("abcde", 2).unwrap_err(),
            "Must be at most 2 characters"
        );
        assert_eq!(
            email("nope").unwrap_err(),
            "Please enter a valid email address"
        );
        assert_eq!(url("nope").unwrap_err(), "Please enter a valid URL");
    }
}
