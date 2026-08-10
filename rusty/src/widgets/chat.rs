//! The chat surface: a scrolling message list with an input box, the messages
//! themselves, a typing indicator and a status line.
//!
//! Four widgets, mapping onto `Ivy.Chat`, `Ivy.ChatMessage`, `Ivy.ChatLoading`
//! and `Ivy.ChatStatus`. The React components already exist under
//! `src/frontend/src/widgets/chat/`; this module is the Rust producer for them.
//!
//! # Divergence from Ivy
//!
//! [`Chat::quick_replies`] is a Rust-side prop only — Ivy's `ChatWidget` reads no
//! such prop, so nothing renders it today. Selecting a quick reply re-uses
//! `on_send` rather than adding a fifth event, because the payload is identical
//! to typing the same text. The E2E renderer in `e2e/app/index.html` implements
//! it that way, so the round trip is exercised even though Ivy ignores the prop.

use crate::shared::{Density, Size};
use crate::views::view::Element;
use rusty_macros::Widget;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Who wrote a [`ChatMessage`].
///
/// Serializes camelCase (`"user"` / `"assistant"`); `shared::ivy_node` title-cases
/// it into the `"User"` / `"Assistant"` that `ChatMessageWidgetProps` declares.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChatSender {
    #[default]
    User,
    Assistant,
}

/// A chat surface: a list of [`ChatMessage`] children plus a composer.
///
/// The children field must stay named `children`: `#[derive(Widget)]` only wires
/// `children_mut` to a field with that name (or one carrying `#[children]`), and
/// without it `Element::assign_ids` never descends into the messages, so the
/// whole subtree loses its IDs and event registrations.
#[derive(Clone, Serialize, Deserialize, Widget)]
pub struct Chat {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    #[children]
    pub children: Vec<Element>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[prop]
    pub streaming: bool,
    /// Suggested replies rendered as buttons. Rust-only; see the module docs.
    #[prop]
    pub quick_replies: Vec<String>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Size>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<Size>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub density: Option<Density>,
    #[event(arg = "value")]
    #[serde(skip)]
    pub on_send: Option<Arc<dyn Fn(String) + Send + Sync>>,
    #[event]
    #[serde(skip)]
    pub on_cancel: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for Chat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chat")
            .field("messages", &self.children.len())
            .field("streaming", &self.streaming)
            .finish()
    }
}

impl Chat {
    pub fn new() -> Self {
        Chat {
            id: None,
            children: Vec::new(),
            placeholder: None,
            streaming: false,
            quick_replies: Vec::new(),
            width: None,
            height: None,
            density: None,
            on_send: None,
            on_cancel: None,
        }
    }

    /// Append one message (or any element Ivy's message list will tolerate).
    pub fn message(mut self, element: impl Into<Element>) -> Self {
        self.children.push(element.into());
        self
    }

    pub fn messages(mut self, elements: Vec<Element>) -> Self {
        self.children.extend(elements);
        self
    }

    pub fn placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = Some(placeholder.to_string());
        self
    }

    /// Mark the assistant as mid-response, which swaps the send button for a
    /// cancel button in Ivy's `ChatWidget`.
    pub fn streaming(mut self, streaming: bool) -> Self {
        self.streaming = streaming;
        self
    }

    pub fn quick_reply(mut self, reply: &str) -> Self {
        self.quick_replies.push(reply.to_string());
        self
    }

    pub fn quick_replies(mut self, replies: Vec<String>) -> Self {
        self.quick_replies.extend(replies);
        self
    }

    pub fn width(mut self, width: Size) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: Size) -> Self {
        self.height = Some(height);
        self
    }

    pub fn density(mut self, density: Density) -> Self {
        self.density = Some(density);
        self
    }

    /// Fires with the submitted text, whether it was typed or picked from
    /// [`Chat::quick_replies`].
    pub fn on_send(mut self, handler: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_send = Some(Arc::new(handler));
        self
    }

    /// Fires when the user interrupts a streaming response.
    pub fn on_cancel(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_cancel = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for Chat {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Chat> for Element {
    fn from(chat: Chat) -> Self {
        chat.into_element()
    }
}

/// One message bubble inside a [`Chat`].
#[derive(Debug, Clone, Serialize, Deserialize, Widget)]
pub struct ChatMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    pub sender: ChatSender,
    #[prop]
    #[children]
    pub children: Vec<Element>,
}

impl ChatMessage {
    pub fn new(sender: ChatSender) -> Self {
        ChatMessage {
            id: None,
            sender,
            children: Vec::new(),
        }
    }

    /// A message from the person using the app.
    pub fn user(content: impl Into<Element>) -> Self {
        ChatMessage::new(ChatSender::User).child(content)
    }

    /// A message from the model.
    pub fn assistant(content: impl Into<Element>) -> Self {
        ChatMessage::new(ChatSender::Assistant).child(content)
    }

    pub fn sender(mut self, sender: ChatSender) -> Self {
        self.sender = sender;
        self
    }

    pub fn child(mut self, element: impl Into<Element>) -> Self {
        self.children.push(element.into());
        self
    }

    pub fn children(mut self, elements: Vec<Element>) -> Self {
        self.children.extend(elements);
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for ChatMessage {
    fn default() -> Self {
        ChatMessage::new(ChatSender::User)
    }
}

impl From<ChatMessage> for Element {
    fn from(message: ChatMessage) -> Self {
        message.into_element()
    }
}

/// The typing indicator, shown as the body of a [`ChatMessage`] while a response
/// is being generated.
///
/// It carries no props at all — Ivy's `ChatLoadingWidget` takes none either. The
/// `id` field is what keeps `#[derive(Widget)]`'s shape checks happy and is what
/// the client keys on.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Widget)]
pub struct ChatLoading {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl ChatLoading {
    pub fn new() -> Self {
        ChatLoading::default()
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl From<ChatLoading> for Element {
    fn from(loading: ChatLoading) -> Self {
        loading.into_element()
    }
}

/// A single shimmering line of status text, for reporting what the assistant is
/// doing between messages.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Widget)]
pub struct ChatStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    pub text: String,
}

impl ChatStatus {
    pub fn new(text: &str) -> Self {
        ChatStatus {
            id: None,
            text: text.to_string(),
        }
    }

    pub fn text(mut self, text: &str) -> Self {
        self.text = text.to_string();
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl From<ChatStatus> for Element {
    fn from(status: ChatStatus) -> Self {
        status.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::{BuildContext, WidgetData};
    use crate::widgets::text::TextBlock;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    #[test]
    fn test_chat_builder_round_trip() {
        let chat = Chat::new()
            .message(ChatMessage::user(TextBlock::new("hi")))
            .message(ChatMessage::assistant(TextBlock::new("hello")))
            .placeholder("Ask something…")
            .streaming(true)
            .quick_reply("Yes")
            .quick_replies(vec!["No".to_string(), "Maybe".to_string()])
            .width(Size::Percent(100.0))
            .height(Size::Px(480.0))
            .density(Density::Compact);

        assert_eq!(chat.children.len(), 2);
        assert_eq!(chat.placeholder.as_deref(), Some("Ask something…"));
        assert!(chat.streaming);
        assert_eq!(chat.quick_replies, vec!["Yes", "No", "Maybe"]);
        assert_eq!(chat.width, Some(Size::Percent(100.0)));
        assert_eq!(chat.height, Some(Size::Px(480.0)));
        assert_eq!(chat.density, Some(Density::Compact));
    }

    #[test]
    fn test_chat_defaults() {
        let chat = Chat::default();
        assert!(chat.children.is_empty());
        assert!(chat.placeholder.is_none());
        assert!(!chat.streaming);
        assert!(chat.quick_replies.is_empty());
        assert!(chat.width.is_none());
        assert!(chat.height.is_none());
        assert!(chat.density.is_none());
    }

    #[test]
    fn test_chat_messages_vec_builder() {
        let chat = Chat::new().messages(vec![
            ChatMessage::user(TextBlock::new("one")).into(),
            ChatMessage::assistant(TextBlock::new("two")).into(),
        ]);
        assert_eq!(chat.children.len(), 2);
    }

    #[test]
    fn test_chat_to_json_keys() {
        let json = Chat::new()
            .message(ChatMessage::user(TextBlock::new("hi")))
            .placeholder("Ask something…")
            .streaming(true)
            .quick_reply("Yes")
            .on_send(|_| {})
            .on_cancel(|| {})
            .to_json();

        assert_eq!(json["type"], "chat");
        assert_eq!(json["placeholder"], "Ask something…");
        assert_eq!(json["streaming"], true);
        assert_eq!(json["quickReplies"], json!(["Yes"]));
        assert_eq!(json["hasOnSend"], true);
        assert_eq!(json["hasOnCancel"], true);
        assert_eq!(json["children"][0]["sender"], "user");
    }

    #[test]
    fn test_chat_json_without_handlers() {
        let json = Chat::new().to_json();
        assert_eq!(json["hasOnSend"], false);
        assert_eq!(json["hasOnCancel"], false);
        assert_eq!(json["streaming"], false);
        assert_eq!(json["quickReplies"], json!([]));
    }

    #[test]
    fn test_chat_assign_ids_descends_into_messages() {
        // The `children` naming rule exists for exactly this: a differently named
        // container gets no `children_mut`, and this assertion is what notices.
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = Chat::new()
            .message(ChatMessage::user(TextBlock::new("hi")))
            .into();
        element.assign_ids(&mut ctx);

        if let Element::Widget(ref w) = element {
            let json = w.to_json();
            assert_eq!(json["id"], "w-0");
            assert_eq!(json["children"][0]["id"], "w-1");
            assert_eq!(json["children"][0]["children"][0]["id"], "w-2");
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_chat_send_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received = Arc::new(Mutex::new(None::<String>));
        let received_clone = received.clone();
        let mut element: Element = Chat::new()
            .on_send(move |value| {
                *received_clone.lock().unwrap() = Some(value);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "send", json!({"value": "hello"})));
        assert_eq!(received.lock().unwrap().as_deref(), Some("hello"));
    }

    #[test]
    fn test_chat_cancel_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let cancels = Arc::new(AtomicUsize::new(0));
        let cancels_clone = cancels.clone();
        let mut element: Element = Chat::new()
            .on_cancel(move || {
                cancels_clone.fetch_add(1, Ordering::SeqCst);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "cancel", serde_json::Value::Null));
        assert_eq!(cancels.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_chat_message_builder_and_json() {
        let message = ChatMessage::assistant(TextBlock::new("done"));
        assert_eq!(message.sender, ChatSender::Assistant);
        assert_eq!(message.children.len(), 1);

        let json = message.to_json();
        assert_eq!(json["type"], "chat_message");
        assert_eq!(json["sender"], "assistant");
        assert_eq!(json["children"][0]["content"], "done");
    }

    #[test]
    fn test_chat_message_defaults_to_user() {
        let message = ChatMessage::default();
        assert_eq!(message.sender, ChatSender::User);
        assert!(message.children.is_empty());

        let switched = ChatMessage::default().sender(ChatSender::Assistant);
        assert_eq!(switched.sender, ChatSender::Assistant);
    }

    #[test]
    fn test_chat_message_children_vec_builder() {
        let message = ChatMessage::new(ChatSender::User)
            .children(vec![TextBlock::new("a").into(), TextBlock::new("b").into()]);
        assert_eq!(message.children.len(), 2);
    }

    #[test]
    fn test_chat_sender_serializes_camel_case() {
        assert_eq!(
            serde_json::to_string(&ChatSender::User).unwrap(),
            "\"user\""
        );
        assert_eq!(
            serde_json::to_string(&ChatSender::Assistant).unwrap(),
            "\"assistant\""
        );
    }

    #[test]
    fn test_chat_loading_json_is_type_and_id_only() {
        let json = ChatLoading::new().to_json();
        assert_eq!(json["type"], "chat_loading");
        // No props at all, matching Ivy's ChatLoadingWidget.
        assert_eq!(json.as_object().unwrap().len(), 2);
        assert!(json["id"].is_null());
    }

    #[test]
    fn test_chat_status_builder_and_json() {
        let status = ChatStatus::new("Thinking").text("Searching");
        assert_eq!(status.text, "Searching");

        let json = status.to_json();
        assert_eq!(json["type"], "chat_status");
        assert_eq!(json["text"], "Searching");
    }

    #[test]
    fn test_chat_widgets_into_element() {
        assert!(matches!(Element::from(Chat::new()), Element::Widget(_)));
        assert!(matches!(
            Element::from(ChatMessage::default()),
            Element::Widget(_)
        ));
        assert!(matches!(
            Element::from(ChatLoading::new()),
            Element::Widget(_)
        ));
        assert!(matches!(
            Element::from(ChatStatus::new("x")),
            Element::Widget(_)
        ));
    }
}
