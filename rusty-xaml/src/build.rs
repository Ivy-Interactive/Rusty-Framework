//! The XAML vocabulary, and how it maps onto `rusty::widgets`.
//!
//! This is the runtime counterpart of `rusty-ivyml`'s `codegen::shape_for`, and
//! it is a table for the same reason: the constructors are not uniform
//! (`Layout::vertical()` vs `TextBlock::new(content)` vs `Card::new()`), `List`
//! attaches children with `.item` rather than `.child`, and several attributes
//! are consumed by the constructor instead of by a builder call. None of that
//! can be derived from a tag name.
//!
//! Two rules hold everywhere:
//!
//! - Nothing is dropped silently. An unknown element, an unknown attribute, an
//!   unparseable value, a child on a leaf — each is an error. A XAML document
//!   that looks styled and renders unstyled is the harder bug to find.
//! - Every XAML spelling has its Rusty spelling as an accepted alias, so a tree
//!   can be written either way (`<StackPanel>` or `<Layout>`, `Spacing` or
//!   `Gap`).

use roxmltree::{Document, Node};

use rusty::views::Element;
use rusty::widgets::badge::BadgeVariant;
use rusty::widgets::button::ButtonVariant;
use rusty::widgets::text::TextVariant;
use rusty::widgets::{
    Badge, Button, Card, Container, Image, Layout, List, ListItem, Progress, Separator, Spacer,
    TextBlock, TextInput,
};

use crate::context::{Handler, XamlContext};
use crate::error::{Position, XamlError};
use crate::value::{self, Site};

/// The namespace `x:Name`, `x:Key` and the other XAML directives live in.
/// Attributes in it are ignored — see the crate docs for why `x:Name` in
/// particular cannot become a widget id.
const XAML_DIRECTIVE_NS: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

/// One attribute, with its value still unresolved.
#[derive(Debug)]
struct Attr {
    name: String,
    raw: String,
    pos: Position,
}

/// An element's attributes, minus the ones a constructor has taken.
///
/// `take` removes, so whatever is left after the constructor runs is exactly the
/// set that must map onto builder calls — the runtime form of `codegen.rs`'s
/// `consumed_by_ctor` list.
#[derive(Debug)]
struct Attrs {
    /// The element's own position, used for errors about a missing attribute.
    pos: Position,
    list: Vec<Attr>,
}

impl Attrs {
    fn take(&mut self, names: &[&str]) -> Option<Attr> {
        let index = self
            .list
            .iter()
            .position(|a| names.contains(&a.name.as_str()))?;
        Some(self.list.remove(index))
    }

    fn require(&mut self, element: &str, names: &[&str]) -> Result<Attr, XamlError> {
        self.take(names).ok_or_else(|| XamlError::MissingAttribute {
            element: element.to_string(),
            attribute: names[0].to_string(),
            pos: self.pos,
        })
    }

    /// The first attribute that was not consumed, for widgets that take none.
    fn any(&self) -> Option<&Attr> {
        self.list.first()
    }
}

/// One parse in progress: the document (for positions) and the runtime data.
struct Build<'a, 'input> {
    doc: &'a Document<'input>,
    ctx: &'a XamlContext,
}

/// Parse the document's root element into a widget tree.
pub(crate) fn build_root(doc: &Document<'_>, ctx: &XamlContext) -> Result<Element, XamlError> {
    // Not `Document::root_element`, which panics when there is no element.
    let root = doc
        .root()
        .children()
        .find(|node| node.is_element())
        .ok_or(XamlError::NoRoot)?;

    Build { doc, ctx }.element(root)
}

impl Build<'_, '_> {
    fn element(&self, node: Node<'_, '_>) -> Result<Element, XamlError> {
        let name = node.tag_name().name();
        let pos = self.position(node.range().start);
        let mut attrs = self.attrs(node, name, pos)?;
        let text = text_content(node);
        let children: Vec<Node<'_, '_>> = node.children().filter(|n| n.is_element()).collect();

        match name {
            "StackPanel" | "Layout" => {
                let layout = match attrs.take(&["Orientation", "Direction"]) {
                    None => Layout::vertical(),
                    Some(attr) => match self.text_of(&attr, name)?.as_str() {
                        "Vertical" => Layout::vertical(),
                        other => {
                            if other == "Horizontal" {
                                Layout::horizontal()
                            } else {
                                return Err(site(name, &attr).unsupported(
                                    other,
                                    "expected `Vertical` or `Horizontal`; \
                                     a grid is `<Grid Columns=\"..\">`",
                                ));
                            }
                        }
                    },
                };
                self.layout(layout, name, attrs, text, &children)
            }
            "Grid" => {
                let attr = attrs.require(name, &["Columns"])?;
                let columns = value::as_usize(&self.resolve(&attr, name)?, site(name, &attr))?;
                self.layout(Layout::grid(columns), name, attrs, text, &children)
            }
            "WrapPanel" => self.layout(
                Layout::horizontal().wrap(true),
                name,
                attrs,
                text,
                &children,
            ),
            "Border" | "Container" => {
                reject_text(name, &text, pos)?;
                let mut widget = Container::new();
                for attr in &attrs.list {
                    widget = self.container_attr(widget, attr, name)?;
                }
                for child in &children {
                    widget = widget.child(self.element(*child)?);
                }
                Ok(widget.into())
            }
            "GroupBox" | "Card" => {
                reject_text(name, &text, pos)?;
                let mut widget = Card::new();
                for attr in &attrs.list {
                    widget = self.card_attr(widget, attr, name)?;
                }
                for child in &children {
                    widget = widget.child(self.element(*child)?);
                }
                Ok(widget.into())
            }
            "ItemsControl" | "ListView" | "List" => {
                reject_text(name, &text, pos)?;
                // `List` holds only `items`; it has no builder to apply.
                if let Some(attr) = attrs.any() {
                    return Err(unknown_attribute(name, &attr.name, attr.pos));
                }
                let mut widget = List::new();
                for child in &children {
                    // `.item`, not `.child`: see `List` in `rusty::widgets::list`.
                    widget = widget.item(self.element(*child)?);
                }
                Ok(widget.into())
            }
            "ListViewItem" | "ListItem" => {
                reject_children(name, &children, pos)?;
                let title = self.content(name, &mut attrs, &["Content", "Title"], &text)?;
                let mut widget = ListItem::new(&title);
                for attr in &attrs.list {
                    widget = self.list_item_attr(widget, attr, name)?;
                }
                Ok(widget.into())
            }
            "TextBlock" => {
                reject_children(name, &children, pos)?;
                let content = self.content(name, &mut attrs, &["Text", "Content"], &text)?;
                let mut widget = TextBlock::new(&content);
                for attr in &attrs.list {
                    widget = self.text_block_attr(widget, attr, name)?;
                }
                Ok(widget.into())
            }
            "Button" => {
                reject_children(name, &children, pos)?;
                let title = self.content(name, &mut attrs, &["Content", "Title"], &text)?;
                let mut widget = Button::new(&title);
                for attr in &attrs.list {
                    widget = self.button_attr(widget, attr, name)?;
                }
                Ok(widget.into())
            }
            "Badge" => {
                reject_children(name, &children, pos)?;
                let label = self.content(name, &mut attrs, &["Content", "Label"], &text)?;
                let mut widget = Badge::new(&label);
                for attr in &attrs.list {
                    widget = self.badge_attr(widget, attr, name)?;
                }
                Ok(widget.into())
            }
            "TextBox" | "TextInput" => {
                reject_children(name, &children, pos)?;
                reject_text(name, &text, pos)?;
                let mut widget = TextInput::new();
                for attr in &attrs.list {
                    widget = self.text_input_attr(widget, attr, name)?;
                }
                Ok(widget.into())
            }
            "Image" => {
                reject_children(name, &children, pos)?;
                reject_text(name, &text, pos)?;
                let source = attrs.require(name, &["Source"])?;
                let mut widget = Image::new(&self.text_of(&source, name)?);
                for attr in &attrs.list {
                    widget = self.image_attr(widget, attr, name)?;
                }
                Ok(widget.into())
            }
            "ProgressBar" | "Progress" => {
                reject_children(name, &children, pos)?;
                reject_text(name, &text, pos)?;
                let attr = attrs.require(name, &["Value"])?;
                let value = value::as_f64(&self.resolve(&attr, name)?, site(name, &attr))?;
                let mut widget = Progress::new(value);
                for attr in &attrs.list {
                    widget = self.progress_attr(widget, attr, name)?;
                }
                Ok(widget.into())
            }
            "Separator" => {
                reject_children(name, &children, pos)?;
                reject_text(name, &text, pos)?;
                let mut widget = match attrs.take(&["Orientation"]) {
                    None => Separator::horizontal(),
                    Some(attr) => match self.text_of(&attr, name)?.as_str() {
                        "Horizontal" => Separator::horizontal(),
                        other => {
                            if other == "Vertical" {
                                Separator::vertical()
                            } else {
                                return Err(site(name, &attr)
                                    .unsupported(other, "expected `Horizontal` or `Vertical`"));
                            }
                        }
                    },
                };
                for attr in &attrs.list {
                    widget = match attr.name.as_str() {
                        "Text" => widget.text(&self.text_of(attr, name)?),
                        _ => return Err(unknown_attribute(name, &attr.name, attr.pos)),
                    };
                }
                Ok(widget.into())
            }
            "Spacer" => {
                reject_children(name, &children, pos)?;
                reject_text(name, &text, pos)?;
                if let Some(attr) = attrs.any() {
                    return Err(unknown_attribute(name, &attr.name, attr.pos));
                }
                Ok(Spacer::new().into())
            }
            other => Err(XamlError::UnknownElement {
                element: other.to_string(),
                pos,
            }),
        }
    }

    /// `StackPanel`, `Grid` and `WrapPanel` differ only in their constructor.
    fn layout(
        &self,
        mut widget: Layout,
        element: &str,
        attrs: Attrs,
        text: Option<String>,
        children: &[Node<'_, '_>],
    ) -> Result<Element, XamlError> {
        reject_text(element, &text, attrs.pos)?;
        for attr in &attrs.list {
            widget = self.layout_attr(widget, attr, element)?;
        }
        for child in children {
            widget = widget.child(self.element(*child)?);
        }
        Ok(widget.into())
    }

    fn layout_attr(&self, w: Layout, attr: &Attr, element: &str) -> Result<Layout, XamlError> {
        let site = site(element, attr);
        let value = self.resolve(attr, element)?;
        Ok(match attr.name.as_str() {
            "Spacing" | "Gap" => w.gap(value::as_f64(&value, site)?),
            "Padding" => w.padding(value::as_f64(&value, site)?),
            "HorizontalAlignment" | "Align" => w.align(value::as_align(&value, site)?),
            "VerticalAlignment" | "Justify" => w.justify(value::as_justify(&value, site)?),
            "Width" => w.width(value::as_size(&value, site)?),
            "Height" => w.height(value::as_size(&value, site)?),
            "Wrap" => w.wrap(value::as_bool(&value, site)?),
            _ => return Err(unknown_attribute(element, &attr.name, attr.pos)),
        })
    }

    fn container_attr(
        &self,
        w: Container,
        attr: &Attr,
        element: &str,
    ) -> Result<Container, XamlError> {
        let site = site(element, attr);
        let value = self.resolve(attr, element)?;
        Ok(match attr.name.as_str() {
            "Padding" => w.padding(value::as_f64(&value, site)?),
            "Width" => w.width(value::as_size(&value, site)?),
            "Height" => w.height(value::as_size(&value, site)?),
            // `Container` is the one widget with a background, so `Background` is
            // honoured here and rejected everywhere else.
            "Background" => w.background(value::as_color(&value, site)?),
            "Border" => w.border(value::as_bool(&value, site)?),
            "Rounded" => w.rounded(value::as_bool(&value, site)?),
            _ => return Err(unknown_attribute(element, &attr.name, attr.pos)),
        })
    }

    fn card_attr(&self, w: Card, attr: &Attr, element: &str) -> Result<Card, XamlError> {
        let site = site(element, attr);
        let value = self.resolve(attr, element)?;
        Ok(match attr.name.as_str() {
            // `Card::new()` takes no argument, so a `GroupBox`'s `Header` is a
            // builder call rather than a constructor argument.
            "Header" | "Title" => w.title(&value::as_text(&value)),
            "Subtitle" => w.subtitle(&value::as_text(&value)),
            "Padding" => w.padding(value::as_f64(&value, site)?),
            _ => return Err(unknown_attribute(element, &attr.name, attr.pos)),
        })
    }

    fn list_item_attr(
        &self,
        w: ListItem,
        attr: &Attr,
        element: &str,
    ) -> Result<ListItem, XamlError> {
        if attr.name == "Click" {
            let handler = self.handler(attr, element)?;
            return Ok(w.on_click(move || handler()));
        }
        let value = self.resolve(attr, element)?;
        Ok(match attr.name.as_str() {
            "Subtitle" => w.subtitle(&value::as_text(&value)),
            "Icon" => w.icon(value::as_text(&value).as_str()),
            _ => return Err(unknown_attribute(element, &attr.name, attr.pos)),
        })
    }

    fn text_block_attr(
        &self,
        w: TextBlock,
        attr: &Attr,
        element: &str,
    ) -> Result<TextBlock, XamlError> {
        let site = site(element, attr);
        let value = self.resolve(attr, element)?;
        Ok(match attr.name.as_str() {
            "Variant" => w.variant(value::as_enum::<TextVariant>(&value, site)?),
            "Foreground" | "Color" => w.color(value::as_color(&value, site)?),
            // `TextBlock::bold` takes no argument, so `FontWeight` accepts the one
            // value it can express rather than a whole `FontWeight` enum.
            "FontWeight" => match value::as_text(&value).as_str() {
                "Bold" => w.bold(),
                other => return Err(site.unsupported(other, "expected `Bold`")),
            },
            "Bold" => {
                if value::as_bool(&value, site)? {
                    w.bold()
                } else {
                    w
                }
            }
            "Italic" => {
                if value::as_bool(&value, site)? {
                    w.italic()
                } else {
                    w
                }
            }
            _ => return Err(unknown_attribute(element, &attr.name, attr.pos)),
        })
    }

    fn button_attr(&self, w: Button, attr: &Attr, element: &str) -> Result<Button, XamlError> {
        if attr.name == "Click" {
            let handler = self.handler(attr, element)?;
            return Ok(w.on_click(move || handler()));
        }
        let site = site(element, attr);
        let value = self.resolve(attr, element)?;
        Ok(match attr.name.as_str() {
            "Variant" => w.variant(value::as_enum::<ButtonVariant>(&value, site)?),
            // XAML says what is enabled; Rusty says what is disabled.
            "IsEnabled" => w.disabled(!value::as_bool(&value, site)?),
            "Disabled" => w.disabled(value::as_bool(&value, site)?),
            "Loading" => w.loading(value::as_bool(&value, site)?),
            "Foreground" | "Color" => w.color(value::as_color(&value, site)?),
            "Icon" => w.icon(value::as_text(&value).as_str()),
            _ => return Err(unknown_attribute(element, &attr.name, attr.pos)),
        })
    }

    fn badge_attr(&self, w: Badge, attr: &Attr, element: &str) -> Result<Badge, XamlError> {
        let site = site(element, attr);
        let value = self.resolve(attr, element)?;
        Ok(match attr.name.as_str() {
            "Variant" => w.variant(value::as_enum::<BadgeVariant>(&value, site)?),
            "Foreground" | "Color" => w.color(value::as_color(&value, site)?),
            _ => return Err(unknown_attribute(element, &attr.name, attr.pos)),
        })
    }

    fn text_input_attr(
        &self,
        w: TextInput,
        attr: &Attr,
        element: &str,
    ) -> Result<TextInput, XamlError> {
        if attr.name == "Changed" {
            let handler = self.handler(attr, element)?;
            // `TextInput::on_change` is `Fn(String)` while a context handler is
            // `Fn()`, so the new text is dropped. A handler that needs the text
            // belongs in Rust, wired with `TextInput::on_change` directly.
            return Ok(w.on_change(move |_| handler()));
        }
        let site = site(element, attr);
        let value = self.resolve(attr, element)?;
        Ok(match attr.name.as_str() {
            "Text" | "Value" => w.value(&value::as_text(&value)),
            "Placeholder" => w.placeholder(&value::as_text(&value)),
            "Label" => w.label(&value::as_text(&value)),
            "IsEnabled" => w.disabled(!value::as_bool(&value, site)?),
            "Disabled" => w.disabled(value::as_bool(&value, site)?),
            "IsReadOnly" | "ReadOnly" => w.read_only(value::as_bool(&value, site)?),
            _ => return Err(unknown_attribute(element, &attr.name, attr.pos)),
        })
    }

    fn image_attr(&self, w: Image, attr: &Attr, element: &str) -> Result<Image, XamlError> {
        let site = site(element, attr);
        let value = self.resolve(attr, element)?;
        Ok(match attr.name.as_str() {
            "Alt" => w.alt(&value::as_text(&value)),
            "Width" => w.width(value::as_size(&value, site)?),
            "Height" => w.height(value::as_size(&value, site)?),
            _ => return Err(unknown_attribute(element, &attr.name, attr.pos)),
        })
    }

    fn progress_attr(
        &self,
        w: Progress,
        attr: &Attr,
        element: &str,
    ) -> Result<Progress, XamlError> {
        let site = site(element, attr);
        let value = self.resolve(attr, element)?;
        Ok(match attr.name.as_str() {
            "Maximum" | "Max" => w.max(value::as_f64(&value, site)?),
            "Label" => w.label(&value::as_text(&value)),
            "Foreground" | "Color" => w.color(value::as_color(&value, site)?),
            _ => return Err(unknown_attribute(element, &attr.name, attr.pos)),
        })
    }

    /// The constructor argument, from an attribute or from text content.
    ///
    /// `<TextBlock>Hello</TextBlock>` and `<TextBlock Text="Hello" />` are the
    /// same tree; both at once is an error rather than a silent winner.
    fn content(
        &self,
        element: &str,
        attrs: &mut Attrs,
        names: &[&str],
        text: &Option<String>,
    ) -> Result<String, XamlError> {
        match (attrs.take(names), text) {
            (Some(attr), Some(_)) => Err(XamlError::DuplicateContent {
                element: element.to_string(),
                attribute: attr.name,
                pos: attr.pos,
            }),
            (Some(attr), None) => self.text_of(&attr, element),
            (None, Some(text)) => {
                // Text content resolves exactly as the attribute would, so
                // `<TextBlock>{Binding Title}</TextBlock>` binds.
                let site = Site {
                    element,
                    attribute: names[0],
                    pos: attrs.pos,
                };
                Ok(value::as_text(&value::resolve(text, self.ctx, site)?))
            }
            (None, None) => Err(XamlError::MissingAttribute {
                element: element.to_string(),
                attribute: names[0].to_string(),
                pos: attrs.pos,
            }),
        }
    }

    fn resolve(&self, attr: &Attr, element: &str) -> Result<serde_json::Value, XamlError> {
        value::resolve(&attr.raw, self.ctx, site(element, attr))
    }

    fn text_of(&self, attr: &Attr, element: &str) -> Result<String, XamlError> {
        Ok(value::as_text(&self.resolve(attr, element)?))
    }

    /// An event attribute's value is a handler *name*, never a binding.
    fn handler(&self, attr: &Attr, element: &str) -> Result<Handler, XamlError> {
        let name = attr.raw.trim();
        self.ctx
            .handler_of(name)
            .ok_or_else(|| XamlError::UnknownHandler {
                element: element.to_string(),
                attribute: attr.name.clone(),
                handler: name.to_string(),
                pos: attr.pos,
            })
    }

    /// Collect the attributes that describe the widget, dropping the XAML
    /// directives that describe the document.
    fn attrs(&self, node: Node<'_, '_>, element: &str, pos: Position) -> Result<Attrs, XamlError> {
        let mut list = Vec::new();

        for attr in node.attributes() {
            // `xmlns` and `xmlns:*` are namespace declarations, which roxmltree
            // does not report as attributes at all.
            if let Some(uri) = attr.namespace() {
                let prefix = node.lookup_prefix(uri);
                if uri == XAML_DIRECTIVE_NS || prefix == Some("x") {
                    continue;
                }
                // Some other prefix, e.g. a design-time `d:DesignHeight`. It is
                // reported rather than ignored: a prefix this crate does not know
                // is a prefix whose effect it cannot honour.
                let qualified = match prefix {
                    Some(prefix) => format!("{}:{}", prefix, attr.name()),
                    None => attr.name().to_string(),
                };
                return Err(unknown_attribute(
                    element,
                    &qualified,
                    self.position(attr.range().start),
                ));
            }

            list.push(Attr {
                name: attr.name().to_string(),
                raw: attr.value().to_string(),
                pos: self.position(attr.range().start),
            });
        }

        Ok(Attrs { pos, list })
    }

    /// A byte offset as a line and column.
    ///
    /// `text_pos_at` rescans the input, so it is called once per element and once
    /// per attribute, never once per error: an error outlives the `Document` that
    /// could resolve its offset, so the position has to be resolved eagerly.
    fn position(&self, offset: usize) -> Position {
        let pos = self.doc.text_pos_at(offset);
        Position::new(pos.row, pos.col)
    }
}

/// The element's text content, or `None` when there is only whitespace.
///
/// Comments and processing instructions are separate node kinds in roxmltree, so
/// they are skipped by construction rather than by filtering.
fn text_content(node: Node<'_, '_>) -> Option<String> {
    let mut text = String::new();
    for child in node.children().filter(|n| n.is_text()) {
        text.push_str(child.text().unwrap_or_default());
    }

    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn reject_text(element: &str, text: &Option<String>, pos: Position) -> Result<(), XamlError> {
    match text {
        None => Ok(()),
        Some(text) => Err(XamlError::UnsupportedValue {
            element: element.to_string(),
            attribute: "text content".to_string(),
            value: text.clone(),
            reason: "text content is only accepted by `TextBlock`, `Button`, `Badge` \
                     and `ListViewItem`"
                .to_string(),
            pos,
        }),
    }
}

fn reject_children(
    element: &str,
    children: &[Node<'_, '_>],
    pos: Position,
) -> Result<(), XamlError> {
    if children.is_empty() {
        Ok(())
    } else {
        Err(XamlError::NoChildrenAllowed {
            element: element.to_string(),
            pos,
        })
    }
}

fn unknown_attribute(element: &str, attribute: &str, pos: Position) -> XamlError {
    XamlError::UnknownAttribute {
        element: element.to_string(),
        attribute: attribute.to_string(),
        pos,
    }
}

fn site<'a>(element: &'a str, attr: &'a Attr) -> Site<'a> {
    Site {
        element,
        attribute: &attr.name,
        pos: attr.pos,
    }
}

#[cfg(test)]
mod tests {
    use crate::{parse, parse_with, XamlContext, XamlError};
    use rusty::views::Element;
    use serde_json::{json, Value};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// The wire form of a tree, which is what a widget actually is.
    fn wire(xaml: &str) -> Value {
        serde_json::to_value(parse(xaml).expect("parses")).expect("serializes")
    }

    fn err(xaml: &str) -> XamlError {
        parse(xaml).expect_err("should not parse")
    }

    #[test]
    fn stack_panel_is_a_vertical_layout_by_default() {
        let json = wire(r#"<StackPanel />"#);
        assert_eq!(json["type"], "layout");
        assert_eq!(json["direction"], "vertical");
    }

    #[test]
    fn stack_panel_orientation_selects_the_constructor() {
        assert_eq!(
            wire(r#"<StackPanel Orientation="Horizontal" />"#)["direction"],
            "horizontal"
        );
        assert_eq!(
            wire(r#"<StackPanel Orientation="Vertical" />"#)["direction"],
            "vertical"
        );
    }

    #[test]
    fn an_unknown_orientation_is_rejected_and_names_the_value() {
        let err = err(r#"<StackPanel Orientation="Diagonal" />"#);
        assert!(matches!(err, XamlError::UnsupportedValue { .. }), "{err:?}");
        assert!(err.to_string().contains("Diagonal"), "{}", err);
    }

    #[test]
    fn layout_is_an_accepted_alias_for_stack_panel() {
        assert_eq!(wire(r#"<Layout />"#), wire(r#"<StackPanel />"#));
    }

    #[test]
    fn grid_takes_its_columns_from_the_constructor() {
        let json = wire(r#"<Grid Columns="3" />"#);
        assert_eq!(json["type"], "layout");
        assert_eq!(json["direction"], "grid");
        assert_eq!(json["columns"], 3);
    }

    #[test]
    fn a_grid_without_columns_is_an_error() {
        let err = err(r#"<Grid />"#);
        assert!(
            matches!(&err, XamlError::MissingAttribute { attribute, .. } if attribute == "Columns"),
            "{err:?}"
        );
    }

    #[test]
    fn wrap_panel_is_a_wrapping_horizontal_layout() {
        let json = wire(r#"<WrapPanel />"#);
        assert_eq!(json["direction"], "horizontal");
        assert_eq!(json["wrap"], true);
    }

    #[test]
    fn border_is_a_container() {
        let json = wire(
            r#"<Border Padding="8" Width="240" Height="50%" Background="Primary"
                       Border="True" Rounded="True" />"#,
        );

        assert_eq!(json["type"], "container");
        assert_eq!(json["padding"], 8.0);
        assert_eq!(json["width"], "240px");
        assert_eq!(json["height"], "50%");
        assert_eq!(json["background"], "primary");
        assert_eq!(json["border"], true);
        assert_eq!(json["rounded"], true);
    }

    #[test]
    fn group_box_is_a_card_and_header_is_its_title() {
        let json = wire(r#"<GroupBox Header="Totals" Subtitle="This month" Padding="12" />"#);

        assert_eq!(json["type"], "card");
        assert_eq!(json["title"], "Totals");
        assert_eq!(json["subtitle"], "This month");
        assert_eq!(json["padding"], 12.0);
    }

    #[test]
    fn items_control_attaches_children_as_items() {
        let json = wire(
            r#"<ItemsControl>
                   <ListViewItem Content="First" Subtitle="1" Icon="mail" />
                   <ListViewItem Content="Second" />
               </ItemsControl>"#,
        );

        assert_eq!(json["type"], "list");
        assert_eq!(json["items"][0]["type"], "list_item");
        assert_eq!(json["items"][0]["title"], "First");
        assert_eq!(json["items"][0]["subtitle"], "1");
        assert_eq!(json["items"][0]["icon"], "mail");
        assert_eq!(json["items"][1]["title"], "Second");
        // Children landed in `items`, not `children`.
        assert!(json["children"].is_null());
    }

    #[test]
    fn list_view_and_list_are_accepted_aliases() {
        assert_eq!(wire(r#"<ListView />"#), wire(r#"<List />"#));
        assert_eq!(wire(r#"<ItemsControl />"#), wire(r#"<List />"#));
    }

    #[test]
    fn list_takes_no_attributes() {
        let err = err(r#"<ListView Padding="4" />"#);
        assert!(
            matches!(&err, XamlError::UnknownAttribute { attribute, .. } if attribute == "Padding"),
            "{err:?}"
        );
    }

    #[test]
    fn text_block_carries_variant_colour_and_weight() {
        // `r##` so the `#` of the hex colour does not close the raw string.
        let json = wire(
            r##"<TextBlock Text="Hi" Variant="Heading1" Foreground="#ff0000" FontWeight="Bold" />"##,
        );

        assert_eq!(json["type"], "text_block");
        assert_eq!(json["content"], "Hi");
        assert_eq!(json["variant"], "heading1");
        assert_eq!(json["color"], "#ff0000");
        assert_eq!(json["bold"], true);
    }

    #[test]
    fn font_weight_accepts_only_bold() {
        let err = err(r#"<TextBlock Text="Hi" FontWeight="SemiBold" />"#);
        assert!(matches!(err, XamlError::UnsupportedValue { .. }), "{err:?}");
        assert!(err.to_string().contains("SemiBold"), "{}", err);
    }

    #[test]
    fn text_content_and_the_text_attribute_are_the_same_tree() {
        assert_eq!(
            wire(r#"<TextBlock>Hello</TextBlock>"#),
            wire(r#"<TextBlock Text="Hello" />"#)
        );
    }

    #[test]
    fn setting_the_content_twice_is_an_error() {
        let err = err(r#"<TextBlock Text="Hello">Hello</TextBlock>"#);
        assert!(matches!(err, XamlError::DuplicateContent { .. }), "{err:?}");
        assert!(err.to_string().contains("TextBlock"), "{}", err);
    }

    #[test]
    fn a_text_block_without_content_is_an_error() {
        let err = err(r#"<TextBlock />"#);
        assert!(
            matches!(&err, XamlError::MissingAttribute { attribute, .. } if attribute == "Text"),
            "{err:?}"
        );
    }

    #[test]
    fn button_maps_content_variant_and_enabled() {
        let json =
            wire(r#"<Button Content="Save" Variant="Ghost" IsEnabled="False" Icon="save" />"#);

        assert_eq!(json["type"], "button");
        assert_eq!(json["title"], "Save");
        assert_eq!(json["variant"], "ghost");
        // `IsEnabled="False"` is `disabled = true`.
        assert_eq!(json["disabled"], true);
        assert_eq!(json["icon"], "save");
    }

    #[test]
    fn text_box_is_a_text_input() {
        let json = wire(
            r#"<TextBox Text="abc" Placeholder="Name" Label="Name" IsReadOnly="True" IsEnabled="False" />"#,
        );

        assert_eq!(json["type"], "text_input");
        assert_eq!(json["value"], "abc");
        assert_eq!(json["placeholder"], "Name");
        assert_eq!(json["label"], "Name");
        assert_eq!(json["readOnly"], true);
        assert_eq!(json["disabled"], true);
    }

    #[test]
    fn image_takes_its_source_from_the_constructor() {
        let json = wire(r#"<Image Source="/logo.png" Alt="Logo" Width="120" Height="Auto" />"#);

        assert_eq!(json["type"], "image");
        assert_eq!(json["src"], "/logo.png");
        assert_eq!(json["alt"], "Logo");
        assert_eq!(json["width"], "120px");
        assert_eq!(json["height"], "auto");
    }

    #[test]
    fn an_image_without_a_source_is_an_error() {
        let err = err(r#"<Image />"#);
        assert!(
            matches!(&err, XamlError::MissingAttribute { attribute, .. } if attribute == "Source"),
            "{err:?}"
        );
    }

    #[test]
    fn progress_bar_takes_its_value_from_the_constructor() {
        let json = wire(r#"<ProgressBar Value="0.4" Maximum="1" Label="Loading" Color="Info" />"#);

        assert_eq!(json["type"], "progress");
        assert_eq!(json["value"], 0.4);
        assert_eq!(json["max"], 1.0);
        assert_eq!(json["label"], "Loading");
        assert_eq!(json["color"], "info");
    }

    #[test]
    fn separator_orientation_selects_the_constructor() {
        assert_eq!(wire(r#"<Separator />"#)["orientation"], "horizontal");
        assert_eq!(
            wire(r#"<Separator Orientation="Vertical" />"#)["orientation"],
            "vertical"
        );
        assert_eq!(wire(r#"<Separator Text="OR" />"#)["text"], "OR");
    }

    #[test]
    fn badge_maps_content_and_variant() {
        let json = wire(r#"<Badge Content="New" Variant="Outline" Color="Success" />"#);

        assert_eq!(json["type"], "badge");
        assert_eq!(json["label"], "New");
        assert_eq!(json["variant"], "outline");
        assert_eq!(json["color"], "success");
    }

    #[test]
    fn spacer_is_a_leaf_with_no_attributes() {
        assert_eq!(wire(r#"<Spacer />"#)["type"], "spacer");

        let err = err(r#"<Spacer Width="4" />"#);
        assert!(matches!(err, XamlError::UnknownAttribute { .. }), "{err:?}");
    }

    #[test]
    fn layout_alignment_uses_xaml_words() {
        let json = wire(
            r#"<StackPanel HorizontalAlignment="Right" VerticalAlignment="Top"
                           Spacing="8" Padding="16" Wrap="True" />"#,
        );

        assert_eq!(json["align"], "end");
        assert_eq!(json["justify"], "start");
        assert_eq!(json["gap"], 8.0);
        assert_eq!(json["padding"], 16.0);
        assert_eq!(json["wrap"], true);
    }

    #[test]
    fn an_unknown_element_names_itself() {
        let err = err(r#"<Canvas />"#);
        assert!(matches!(err, XamlError::UnknownElement { .. }), "{err:?}");
        assert!(err.to_string().contains("Canvas"), "{}", err);
    }

    #[test]
    fn an_unknown_attribute_names_itself_and_its_element() {
        let err = err(r#"<StackPanel Margin="8" />"#);
        assert!(matches!(err, XamlError::UnknownAttribute { .. }), "{err:?}");

        let message = err.to_string();
        assert!(message.contains("Margin"), "{}", message);
        assert!(message.contains("StackPanel"), "{}", message);
    }

    #[test]
    fn background_is_rejected_where_no_widget_can_honour_it() {
        let err = err(r#"<StackPanel Background="Primary" />"#);
        assert!(
            matches!(&err, XamlError::UnknownAttribute { attribute, .. } if attribute == "Background"),
            "{err:?}"
        );
    }

    #[test]
    fn children_on_a_leaf_are_an_error() {
        let err = err(r#"<TextBlock Text="Hi"><Spacer /></TextBlock>"#);
        assert!(
            matches!(err, XamlError::NoChildrenAllowed { .. }),
            "{err:?}"
        );
        assert!(err.to_string().contains("TextBlock"), "{}", err);
    }

    #[test]
    fn text_content_on_a_container_is_an_error_not_a_dropped_node() {
        let err = err(r#"<StackPanel>stray</StackPanel>"#);
        assert!(matches!(err, XamlError::UnsupportedValue { .. }), "{err:?}");
        assert!(err.to_string().contains("stray"), "{}", err);
    }

    #[test]
    fn whitespace_between_tags_is_ignorable() {
        let spaced = wire("<StackPanel>\n    <Spacer />\n</StackPanel>");
        let tight = wire("<StackPanel><Spacer /></StackPanel>");
        assert_eq!(spaced, tight);
    }

    #[test]
    fn a_click_resolves_through_the_context() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let ctx = XamlContext::new().handler("OnSave", move || {
            counter.fetch_add(1, Ordering::SeqCst);
        });

        let element = parse_with(r#"<Button Content="Save" Click="OnSave" />"#, &ctx).unwrap();
        let json = serde_json::to_value(&element).unwrap();
        assert_eq!(json["hasOnClick"], true);

        fire_click(&element);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_missing_handler_is_an_error() {
        let err = parse(r#"<Button Content="Save" Click="OnSave" />"#).unwrap_err();
        assert!(
            matches!(&err, XamlError::UnknownHandler { handler, .. } if handler == "OnSave"),
            "{err:?}"
        );
        assert!(err.to_string().contains("OnSave"), "{}", err);
    }

    #[test]
    fn a_list_item_click_and_a_text_box_change_both_resolve() {
        let calls = Arc::new(AtomicUsize::new(0));
        let on_pick = calls.clone();
        let on_type = calls.clone();
        let ctx = XamlContext::new()
            .handler("OnPick", move || {
                on_pick.fetch_add(1, Ordering::SeqCst);
            })
            .handler("OnType", move || {
                on_type.fetch_add(10, Ordering::SeqCst);
            });

        let item = parse_with(r#"<ListViewItem Content="One" Click="OnPick" />"#, &ctx).unwrap();
        assert_eq!(serde_json::to_value(&item).unwrap()["hasOnClick"], true);

        let input = parse_with(r#"<TextBox Changed="OnType" />"#, &ctx).unwrap();
        assert_eq!(serde_json::to_value(&input).unwrap()["hasOnChange"], true);

        fire_click(&item);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn bindings_reach_typed_builder_slots() {
        let ctx = XamlContext::new()
            .value("Title", "Dashboard")
            .value("Columns", 2)
            .value("Gap", 12.5);

        let json = serde_json::to_value(
            parse_with(
                r#"<Grid Columns="{Binding Columns}" Spacing="{Binding Gap}">
                       <TextBlock Text="{Binding Title}" />
                   </Grid>"#,
                &ctx,
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(json["columns"], 2);
        assert_eq!(json["gap"], 12.5);
        assert_eq!(json["children"][0]["content"], "Dashboard");
    }

    #[test]
    fn xaml_directives_and_namespaces_are_ignored() {
        let decorated = wire(
            r#"<StackPanel xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
                           xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
                           x:Name="root">
                   <!-- a comment -->
                   <TextBlock x:Uid="t" Text="Hi" />
               </StackPanel>"#,
        );

        assert_eq!(
            decorated,
            wire(r#"<StackPanel><TextBlock Text="Hi" /></StackPanel>"#)
        );
    }

    #[test]
    fn an_unrecognised_prefix_is_reported_rather_than_ignored() {
        let err = err(
            r#"<StackPanel xmlns:d="http://schemas.microsoft.com/expression/blend/2008"
                           d:DesignHeight="400" />"#,
        );

        assert!(
            matches!(&err, XamlError::UnknownAttribute { attribute, .. } if attribute == "d:DesignHeight"),
            "{err:?}"
        );
    }

    #[test]
    fn errors_carry_the_line_and_column_of_the_offending_markup() {
        let err = err("<StackPanel>\n    <Canvas />\n</StackPanel>");
        assert!(err.to_string().contains("2:5"), "{}", err);
    }

    #[test]
    fn the_tree_matches_the_equivalent_builder_chain() {
        use rusty::widgets::{Layout, TextBlock};

        let parsed =
            parse(r#"<StackPanel Spacing="8"><TextBlock Text="Hi" /></StackPanel>"#).unwrap();
        let built: Element = Layout::vertical()
            .gap(8.0)
            .child(TextBlock::new("Hi"))
            .into();

        assert_eq!(
            serde_json::to_value(&parsed).unwrap(),
            serde_json::to_value(&built).unwrap()
        );
    }

    #[test]
    fn a_widget_serializes_with_its_kind_tag() {
        // Guards the assumption every other test here rests on: `Element`'s
        // `kind` tag and the widget's own props share one JSON object.
        assert_eq!(
            wire(r#"<Spacer />"#),
            json!({"kind": "widget", "type": "spacer", "id": null})
        );
    }

    /// Assign ids, then invoke the widget's registered `click` handler.
    fn fire_click(element: &Element) {
        use rusty::hooks::hook_store::HookStore;
        use rusty::views::BuildContext;

        let mut element = element.clone();
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        element.assign_ids(&mut ctx);

        let id = serde_json::to_value(&element).unwrap()["id"]
            .as_str()
            .expect("id was assigned")
            .to_string();

        assert!(
            ctx.event_registry_mut().dispatch(&id, "click", Value::Null),
            "no `click` handler was registered"
        );
    }
}
