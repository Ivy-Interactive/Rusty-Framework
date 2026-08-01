use crate::core::event_registry::EventRegistry;
use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// How the two revisions are laid out against each other.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiffViewType {
    #[default]
    Unified,
    Split,
}

/// Renders a unified diff string, optionally side by side.
///
/// The widget carries the diff text verbatim — parsing and syntax highlighting
/// belong to the frontend.
#[derive(Clone, Serialize, Deserialize)]
pub struct DiffView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
    pub view_type: DiffViewType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_revision: Option<String>,
    pub word_wrap: bool,
    pub collapsible: bool,
    pub default_collapsed: bool,
    #[serde(skip)]
    pub on_line_click: Option<Arc<dyn Fn(usize) + Send + Sync>>,
}

impl std::fmt::Debug for DiffView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiffView")
            .field("view_type", &self.view_type)
            .field("language", &self.language)
            .finish()
    }
}

impl DiffView {
    pub fn new() -> Self {
        DiffView {
            id: None,
            diff: None,
            view_type: DiffViewType::Unified,
            language: None,
            old_revision: None,
            new_revision: None,
            word_wrap: false,
            collapsible: false,
            default_collapsed: false,
            on_line_click: None,
        }
    }

    pub fn diff(mut self, diff: &str) -> Self {
        self.diff = Some(diff.to_string());
        self
    }

    pub fn view_type(mut self, view_type: DiffViewType) -> Self {
        self.view_type = view_type;
        self
    }

    pub fn split(mut self) -> Self {
        self.view_type = DiffViewType::Split;
        self
    }

    pub fn unified(mut self) -> Self {
        self.view_type = DiffViewType::Unified;
        self
    }

    pub fn language(mut self, language: &str) -> Self {
        self.language = Some(language.to_string());
        self
    }

    pub fn old_revision(mut self, name: &str) -> Self {
        self.old_revision = Some(name.to_string());
        self
    }

    pub fn new_revision(mut self, name: &str) -> Self {
        self.new_revision = Some(name.to_string());
        self
    }

    pub fn word_wrap(mut self, word_wrap: bool) -> Self {
        self.word_wrap = word_wrap;
        self
    }

    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    pub fn default_collapsed(mut self, collapsed: bool) -> Self {
        self.default_collapsed = collapsed;
        self
    }

    pub fn on_line_click(mut self, handler: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_line_click = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for DiffView {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetData for DiffView {
    fn widget_type(&self) -> &str {
        "diff_view"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "diff_view",
            "id": self.id,
            "diff": self.diff,
            "viewType": self.view_type,
            "language": self.language,
            "oldRevision": self.old_revision,
            "newRevision": self.new_revision,
            "wordWrap": self.word_wrap,
            "collapsible": self.collapsible,
            "defaultCollapsed": self.default_collapsed,
            "hasOnLineClick": self.on_line_click.is_some(),
        })
    }

    fn clone_box(&self) -> Box<dyn WidgetData> {
        Box::new(self.clone())
    }

    fn assign_id(&mut self, id: String) {
        self.id = Some(id);
    }

    fn get_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    fn register_events(&self, widget_id: &str, registry: &mut EventRegistry) {
        if let Some(handler) = &self.on_line_click {
            let handler = handler.clone();
            registry.register(
                widget_id,
                "lineclick",
                Arc::new(move |args| {
                    if let Some(line) = args.get("line").and_then(|v| v.as_u64()) {
                        handler(line as usize);
                    }
                }),
            );
        }
    }
}

impl From<DiffView> for Element {
    fn from(view: DiffView) -> Self {
        view.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::BuildContext;
    use std::sync::Mutex;

    const SAMPLE_DIFF: &str = "@@ -1,2 +1,2 @@\n-old\n+new\n";

    #[test]
    fn test_diff_view_builder_round_trip() {
        let view = DiffView::new()
            .diff(SAMPLE_DIFF)
            .split()
            .language("rust")
            .old_revision("HEAD~1")
            .new_revision("HEAD")
            .word_wrap(true)
            .collapsible(true)
            .default_collapsed(true);

        assert_eq!(view.diff.as_deref(), Some(SAMPLE_DIFF));
        assert_eq!(view.view_type, DiffViewType::Split);
        assert_eq!(view.language.as_deref(), Some("rust"));
        assert_eq!(view.old_revision.as_deref(), Some("HEAD~1"));
        assert_eq!(view.new_revision.as_deref(), Some("HEAD"));
        assert!(view.word_wrap);
        assert!(view.collapsible);
        assert!(view.default_collapsed);
    }

    #[test]
    fn test_diff_view_defaults() {
        let view = DiffView::default();
        assert_eq!(view.view_type, DiffViewType::Unified);
        assert!(!view.word_wrap);
        assert!(!view.collapsible);
        assert!(!view.default_collapsed);
        assert!(view.diff.is_none());
    }

    #[test]
    fn test_diff_view_unified_resets_view_type() {
        let view = DiffView::new().split().unified();
        assert_eq!(view.view_type, DiffViewType::Unified);
    }

    #[test]
    fn test_diff_view_to_json_keys() {
        let json = DiffView::new()
            .diff(SAMPLE_DIFF)
            .view_type(DiffViewType::Split)
            .on_line_click(|_| {})
            .to_json();

        assert_eq!(json["type"], "diff_view");
        assert_eq!(json["diff"], SAMPLE_DIFF);
        assert_eq!(json["viewType"], "split");
        assert_eq!(json["wordWrap"], false);
        assert_eq!(json["hasOnLineClick"], true);
    }

    #[test]
    fn test_diff_view_json_without_handler() {
        assert_eq!(DiffView::new().to_json()["hasOnLineClick"], false);
    }

    #[test]
    fn test_diff_view_assign_ids() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let mut element: Element = DiffView::new().into();
        element.assign_ids(&mut ctx);
        if let Element::Widget(ref w) = element {
            assert_eq!(w.get_id(), Some("w-0"));
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_diff_view_line_click_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received = Arc::new(Mutex::new(None::<usize>));
        let received_clone = received.clone();
        let mut element: Element = DiffView::new()
            .on_line_click(move |line| {
                *received_clone.lock().unwrap() = Some(line);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "lineclick", json!({"line": 12})));
        assert_eq!(*received.lock().unwrap(), Some(12));
    }
}
