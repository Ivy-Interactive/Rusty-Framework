use crate::shared::Color;
use crate::views::view::Element;
use rusty_macros::Widget;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// How the terminal cursor is drawn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CursorStyle {
    #[default]
    Block,
    Underline,
    Bar,
}

/// The terminal's dimensions, delivered to `Terminal::on_resize`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

/// An interactive terminal emulator surface.
///
/// Ivy's `Stream` property (an `IWriteStream<byte[]>` for pushing output) is not
/// ported — Rusty has no stream abstraction. Content is seeded through
/// `initial_content` and updated by rebuilding the widget.
#[derive(Clone, Serialize, Deserialize, Widget)]
pub struct Terminal {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cols: Option<u16>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<u16>,
    #[prop]
    pub cursor_blink: bool,
    #[prop]
    pub cursor_style: CursorStyle,
    #[prop]
    pub scrollback: u32,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_content: Option<String>,
    #[prop]
    pub closed: bool,
    #[prop]
    pub allow_clipboard: bool,
    #[prop]
    pub auto_focus: bool,
    #[prop]
    pub loading: bool,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loading_text: Option<String>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Color>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<Color>,
    #[event(arg = "data")]
    #[serde(skip)]
    pub on_input: Option<Arc<dyn Fn(String) + Send + Sync>>,
    #[event(payload)]
    #[serde(skip)]
    pub on_resize: Option<Arc<dyn Fn(TerminalSize) + Send + Sync>>,
    #[event(arg = "url")]
    #[serde(skip)]
    pub on_link_click: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl std::fmt::Debug for Terminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Terminal")
            .field("cols", &self.cols)
            .field("rows", &self.rows)
            .field("closed", &self.closed)
            .finish()
    }
}

impl Terminal {
    pub fn new() -> Self {
        Terminal {
            id: None,
            cols: None,
            rows: None,
            cursor_blink: true,
            cursor_style: CursorStyle::Block,
            scrollback: 1000,
            initial_content: None,
            closed: false,
            allow_clipboard: true,
            auto_focus: true,
            loading: false,
            loading_text: None,
            background: None,
            foreground: None,
            on_input: None,
            on_resize: None,
            on_link_click: None,
        }
    }

    pub fn cols(mut self, cols: u16) -> Self {
        self.cols = Some(cols);
        self
    }

    pub fn rows(mut self, rows: u16) -> Self {
        self.rows = Some(rows);
        self
    }

    pub fn cursor_blink(mut self, blink: bool) -> Self {
        self.cursor_blink = blink;
        self
    }

    pub fn cursor_style(mut self, style: CursorStyle) -> Self {
        self.cursor_style = style;
        self
    }

    pub fn scrollback(mut self, lines: u32) -> Self {
        self.scrollback = lines;
        self
    }

    pub fn initial_content(mut self, content: &str) -> Self {
        self.initial_content = Some(content.to_string());
        self
    }

    pub fn closed(mut self, closed: bool) -> Self {
        self.closed = closed;
        self
    }

    pub fn allow_clipboard(mut self, allow: bool) -> Self {
        self.allow_clipboard = allow;
        self
    }

    pub fn auto_focus(mut self, auto_focus: bool) -> Self {
        self.auto_focus = auto_focus;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Show the loading state with an accompanying message.
    pub fn loading_text(mut self, text: &str) -> Self {
        self.loading = true;
        self.loading_text = Some(text.to_string());
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn foreground(mut self, color: Color) -> Self {
        self.foreground = Some(color);
        self
    }

    pub fn on_input(mut self, handler: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_input = Some(Arc::new(handler));
        self
    }

    pub fn on_resize(mut self, handler: impl Fn(TerminalSize) + Send + Sync + 'static) -> Self {
        self.on_resize = Some(Arc::new(handler));
        self
    }

    pub fn on_link_click(mut self, handler: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_link_click = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Terminal> for Element {
    fn from(terminal: Terminal) -> Self {
        terminal.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::{BuildContext, WidgetData};
    use serde_json::json;
    use std::sync::Mutex;

    #[test]
    fn test_terminal_builder_round_trip() {
        let terminal = Terminal::new()
            .cols(120)
            .rows(40)
            .cursor_blink(false)
            .cursor_style(CursorStyle::Bar)
            .scrollback(5000)
            .initial_content("$ ls\n")
            .closed(true)
            .allow_clipboard(false)
            .auto_focus(false)
            .background(Color::hex("#000000"))
            .foreground(Color::hex("#ffffff"));

        assert_eq!(terminal.cols, Some(120));
        assert_eq!(terminal.rows, Some(40));
        assert!(!terminal.cursor_blink);
        assert_eq!(terminal.cursor_style, CursorStyle::Bar);
        assert_eq!(terminal.scrollback, 5000);
        assert_eq!(terminal.initial_content.as_deref(), Some("$ ls\n"));
        assert!(terminal.closed);
        assert!(!terminal.allow_clipboard);
        assert!(!terminal.auto_focus);
        assert_eq!(terminal.background, Some(Color::hex("#000000")));
        assert_eq!(terminal.foreground, Some(Color::hex("#ffffff")));
    }

    #[test]
    fn test_terminal_defaults() {
        let terminal = Terminal::default();
        assert!(terminal.cursor_blink);
        assert_eq!(terminal.cursor_style, CursorStyle::Block);
        assert_eq!(terminal.scrollback, 1000);
        assert!(!terminal.closed);
        assert!(terminal.allow_clipboard);
        assert!(terminal.auto_focus);
        assert!(!terminal.loading);
        assert!(terminal.loading_text.is_none());
    }

    #[test]
    fn test_terminal_loading_text_sets_loading() {
        let terminal = Terminal::new().loading_text("Connecting…");
        assert!(terminal.loading);
        assert_eq!(terminal.loading_text.as_deref(), Some("Connecting…"));
    }

    #[test]
    fn test_terminal_to_json_keys() {
        let json = Terminal::new()
            .cols(80)
            .rows(24)
            .cursor_style(CursorStyle::Underline)
            .on_input(|_| {})
            .on_resize(|_| {})
            .on_link_click(|_| {})
            .to_json();

        assert_eq!(json["type"], "terminal");
        assert_eq!(json["cols"], 80);
        assert_eq!(json["rows"], 24);
        assert_eq!(json["cursorStyle"], "underline");
        assert_eq!(json["scrollback"], 1000);
        assert_eq!(json["allowClipboard"], true);
        assert_eq!(json["hasOnInput"], true);
        assert_eq!(json["hasOnResize"], true);
        assert_eq!(json["hasOnLinkClick"], true);
    }

    #[test]
    fn test_terminal_json_without_handlers() {
        let json = Terminal::new().to_json();
        assert_eq!(json["hasOnInput"], false);
        assert_eq!(json["hasOnResize"], false);
        assert_eq!(json["hasOnLinkClick"], false);
    }

    #[test]
    fn test_terminal_assign_ids() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let mut element: Element = Terminal::new().into();
        element.assign_ids(&mut ctx);
        if let Element::Widget(ref w) = element {
            assert_eq!(w.get_id(), Some("w-0"));
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_terminal_input_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received = Arc::new(Mutex::new(None::<String>));
        let received_clone = received.clone();
        let mut element: Element = Terminal::new()
            .on_input(move |data| {
                *received_clone.lock().unwrap() = Some(data);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "input", json!({"data": "echo hi\r"})));
        assert_eq!(received.lock().unwrap().as_deref(), Some("echo hi\r"));
    }

    #[test]
    fn test_terminal_resize_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received = Arc::new(Mutex::new(None::<TerminalSize>));
        let received_clone = received.clone();
        let mut element: Element = Terminal::new()
            .on_resize(move |size| {
                *received_clone.lock().unwrap() = Some(size);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "resize", json!({"cols": 100, "rows": 30})));
        assert_eq!(
            *received.lock().unwrap(),
            Some(TerminalSize {
                cols: 100,
                rows: 30
            })
        );
    }

    #[test]
    fn test_terminal_link_click_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received = Arc::new(Mutex::new(None::<String>));
        let received_clone = received.clone();
        let mut element: Element = Terminal::new()
            .on_link_click(move |url| {
                *received_clone.lock().unwrap() = Some(url);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "linkclick", json!({"url": "https://example.com"})));
        assert_eq!(
            received.lock().unwrap().as_deref(),
            Some("https://example.com")
        );
    }
}
