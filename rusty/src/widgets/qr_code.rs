use crate::shared::Color;
use crate::views::view::{Element, WidgetData};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// How much redundancy the generated QR code carries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QrErrorCorrectionLevel {
    #[default]
    Low,
    Medium,
    Quartile,
    High,
}

/// Carries a string for the frontend to render as a QR code.
///
/// No encoding happens here — the widget ships `value` and the rendering
/// parameters, so no QR encoding crate is pulled in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QrCode {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixel_size: Option<u32>,
    pub error_correction_level: QrErrorCorrectionLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<Color>,
}

impl QrCode {
    pub fn new(value: &str) -> Self {
        QrCode {
            id: None,
            value: value.to_string(),
            pixel_size: None,
            error_correction_level: QrErrorCorrectionLevel::Low,
            background: None,
            foreground: None,
        }
    }

    pub fn value(mut self, value: &str) -> Self {
        self.value = value.to_string();
        self
    }

    pub fn pixel_size(mut self, size: u32) -> Self {
        self.pixel_size = Some(size);
        self
    }

    pub fn error_correction_level(mut self, level: QrErrorCorrectionLevel) -> Self {
        self.error_correction_level = level;
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

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl WidgetData for QrCode {
    fn widget_type(&self) -> &str {
        "qr_code"
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": "qr_code",
            "id": self.id,
            "value": self.value,
            "pixelSize": self.pixel_size,
            "errorCorrectionLevel": self.error_correction_level,
            "background": self.background,
            "foreground": self.foreground,
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
}

impl From<QrCode> for Element {
    fn from(code: QrCode) -> Self {
        code.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::shared::NamedColor;
    use crate::views::view::BuildContext;

    #[test]
    fn test_qr_code_builder_round_trip() {
        let code = QrCode::new("https://example.com")
            .pixel_size(8)
            .error_correction_level(QrErrorCorrectionLevel::High)
            .background(Color::hex("#ffffff"))
            .foreground(Color::Named(NamedColor::Black));

        assert_eq!(code.value, "https://example.com");
        assert_eq!(code.pixel_size, Some(8));
        assert_eq!(code.error_correction_level, QrErrorCorrectionLevel::High);
        assert_eq!(code.background, Some(Color::hex("#ffffff")));
        assert_eq!(code.foreground, Some(Color::Named(NamedColor::Black)));
    }

    #[test]
    fn test_qr_code_defaults() {
        let code = QrCode::new("hello");
        assert_eq!(code.error_correction_level, QrErrorCorrectionLevel::Low);
        assert!(code.pixel_size.is_none());
        assert!(code.background.is_none());
        assert!(code.foreground.is_none());
    }

    #[test]
    fn test_qr_code_value_setter_replaces_value() {
        let code = QrCode::new("first").value("second");
        assert_eq!(code.value, "second");
    }

    #[test]
    fn test_qr_code_to_json_keys() {
        let json = QrCode::new("payload")
            .pixel_size(4)
            .error_correction_level(QrErrorCorrectionLevel::Quartile)
            .to_json();

        assert_eq!(json["type"], "qr_code");
        assert_eq!(json["value"], "payload");
        assert_eq!(json["pixelSize"], 4);
        assert_eq!(json["errorCorrectionLevel"], "quartile");
    }

    #[test]
    fn test_qr_code_assign_ids() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let mut element: Element = QrCode::new("x").into();
        element.assign_ids(&mut ctx);
        if let Element::Widget(ref w) = element {
            assert_eq!(w.get_id(), Some("w-0"));
            assert_eq!(w.to_json()["id"], "w-0");
        } else {
            panic!("Expected Element::Widget");
        }
    }
}
