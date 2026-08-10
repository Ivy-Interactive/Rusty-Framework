//! The one error type the crate returns.
//!
//! Unlike `rusty-ivyml`, which reports a mistake as a `syn::Error` carrying a
//! span that `rustc` renders, this crate sees its input at runtime. Every
//! variant that refers to markup therefore carries the offending element name
//! and a [`Position`], resolved from the byte offset with
//! `roxmltree::Document::text_pos_at` while the document is still in scope.

use std::fmt;
use std::path::PathBuf;

/// A line and column in the parsed document, both 1-based.
///
/// Stored rather than the raw byte offset, because the offset is only meaningful
/// next to the `roxmltree::Document` that produced it, and an error outlives the
/// document it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

impl Position {
    pub fn new(line: u32, column: u32) -> Self {
        Position { line, column }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Everything that can go wrong turning XAML into a widget tree.
///
/// Not `PartialEq`: [`XamlError::Xml`] wraps `roxmltree::Error`, which is not
/// comparable. Assert on `matches!` and on the `Display` string instead.
#[derive(Debug)]
pub enum XamlError {
    /// The input is not well-formed XML. `roxmltree`'s own message already
    /// carries a line and column.
    Xml(roxmltree::Error),
    /// A `.xaml` file could not be read.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The document contains no element. XML makes this all but unreachable —
    /// `roxmltree` rejects an empty document itself — so this guards the one
    /// path where it would otherwise panic.
    NoRoot,
    /// An element name outside the vocabulary in [`crate::build`].
    UnknownElement { element: String, pos: Position },
    /// An attribute that maps onto no builder of that widget.
    UnknownAttribute {
        element: String,
        attribute: String,
        pos: Position,
    },
    /// An attribute the constructor needs, absent.
    MissingAttribute {
        element: String,
        attribute: String,
        pos: Position,
    },
    /// The attribute exists, but its value cannot be coerced to what the
    /// builder takes. `reason` is the coercion's own complaint — for an enum it
    /// is serde's, which lists the accepted variants.
    UnsupportedValue {
        element: String,
        attribute: String,
        value: String,
        reason: String,
        pos: Position,
    },
    /// A `{..}` markup extension other than `{Binding ..}`.
    UnsupportedMarkupExtension {
        element: String,
        attribute: String,
        value: String,
        pos: Position,
    },
    /// `{Binding X}` where `X` is absent from the [`crate::XamlContext`].
    UnresolvedBinding {
        element: String,
        attribute: String,
        path: String,
        pos: Position,
    },
    /// An event attribute naming a handler absent from the
    /// [`crate::XamlContext`].
    UnknownHandler {
        element: String,
        attribute: String,
        handler: String,
        pos: Position,
    },
    /// The constructor argument was given twice: once as an attribute and once
    /// as text content.
    DuplicateContent {
        element: String,
        attribute: String,
        pos: Position,
    },
    /// A leaf element with element children.
    NoChildrenAllowed { element: String, pos: Position },
}

impl fmt::Display for XamlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XamlError::Xml(err) => write!(f, "malformed XAML: {}", err),
            XamlError::Io { path, source } => {
                write!(f, "cannot read `{}`: {}", path.display(), source)
            }
            XamlError::NoRoot => write!(f, "the document contains no root element"),
            XamlError::UnknownElement { element, pos } => {
                write!(f, "unknown XAML element `<{}>` at {}", element, pos)
            }
            XamlError::UnknownAttribute {
                element,
                attribute,
                pos,
            } => write!(
                f,
                "unknown attribute `{}` on `<{}>` at {}",
                attribute, element, pos
            ),
            XamlError::MissingAttribute {
                element,
                attribute,
                pos,
            } => write!(
                f,
                "`<{}>` requires `{}`, missing at {}",
                element, attribute, pos
            ),
            XamlError::UnsupportedValue {
                element,
                attribute,
                value,
                reason,
                pos,
            } => write!(
                f,
                "`{}=\"{}\"` on `<{}>` at {}: {}",
                attribute, value, element, pos, reason
            ),
            XamlError::UnsupportedMarkupExtension {
                element,
                attribute,
                value,
                pos,
            } => write!(
                f,
                "unsupported markup extension `{}` in `{}` on `<{}>` at {}; \
                 only `{{Binding ..}}` is supported",
                value, attribute, element, pos
            ),
            XamlError::UnresolvedBinding {
                element,
                attribute,
                path,
                pos,
            } => write!(
                f,
                "unresolved binding `{}` in `{}` on `<{}>` at {}; \
                 supply it with `XamlContext::value`",
                path, attribute, element, pos
            ),
            XamlError::UnknownHandler {
                element,
                attribute,
                handler,
                pos,
            } => write!(
                f,
                "unknown handler `{}` for `{}` on `<{}>` at {}; \
                 supply it with `XamlContext::handler`",
                handler, attribute, element, pos
            ),
            XamlError::DuplicateContent {
                element,
                attribute,
                pos,
            } => write!(
                f,
                "`<{}>` at {} sets its content twice: as `{}` and as text content",
                element, pos, attribute
            ),
            XamlError::NoChildrenAllowed { element, pos } => {
                write!(f, "`<{}>` at {} does not accept children", element, pos)
            }
        }
    }
}

impl std::error::Error for XamlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            XamlError::Xml(err) => Some(err),
            XamlError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<roxmltree::Error> for XamlError {
    fn from(err: roxmltree::Error) -> Self {
        XamlError::Xml(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_display_includes_line_and_column() {
        let err: XamlError = roxmltree::Document::parse("<Grid>\n  <StackPanel>\n</Grid>")
            .unwrap_err()
            .into();
        let message = err.to_string();

        assert!(message.starts_with("malformed XAML: "), "{}", message);
        assert!(message.contains("3:1"), "{}", message);
    }

    #[test]
    fn unknown_element_display_names_element_and_position() {
        let err = XamlError::UnknownElement {
            element: "Canvas".to_string(),
            pos: Position::new(4, 7),
        };

        assert_eq!(err.to_string(), "unknown XAML element `<Canvas>` at 4:7");
    }

    #[test]
    fn io_display_names_the_path() {
        let err = XamlError::Io {
            path: PathBuf::from("/tmp/missing.xaml"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"),
        };

        let message = err.to_string();
        assert!(message.contains("/tmp/missing.xaml"), "{}", message);
        assert!(message.contains("no such file"), "{}", message);
    }

    #[test]
    fn error_source_is_the_wrapped_cause() {
        use std::error::Error;

        let xml: XamlError = roxmltree::Document::parse("<").unwrap_err().into();
        assert!(xml.source().is_some());

        let no_root = XamlError::NoRoot;
        assert!(no_root.source().is_none());
    }
}
