//! Capture inputs that read from device hardware: the microphone, the camera and
//! a pointer-drawn signature pad.
//!
//! These map onto `Ivy.AudioInput`, `Ivy.CameraInput` and `Ivy.SignatureInput`.
//! They are grouped in one module because they share a shape — a browser capture
//! API produces a blob, and the blob reaches the server as a base64 data URL.
//!
//! # Two ways to receive a capture
//!
//! Ivy's audio and camera inputs take an `uploadUrl` and POST the recording there
//! themselves (via `uploadFile` in `filePicker/shared.ts`), firing no event at
//! all. [`AudioInput::upload_url`] and [`CameraInput::upload_url`] exist to drive
//! that path, and are the right choice for large recordings.
//!
//! `on_capture` is the Rust-native path instead: the client hands the encoded
//! recording straight to the handler over the event socket, with no HTTP upload.
//! Rusty ships no upload endpoint today, so `on_capture` is the only path that
//! works out of the box; `upload_url` is here so an app pointing at an existing
//! service does not have to route around the widget.

use crate::shared::{Color, Density, Size};
use crate::views::view::Element;
use rusty_macros::Widget;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Which camera to open.
///
/// Deliberately *not* title-cased on its way to Ivy (see `shared::ivy_node`):
/// the value is passed to `getUserMedia`, which only accepts the lowercase
/// `"user"` / `"environment"` spellings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FacingMode {
    /// The front-facing (selfie) camera.
    #[default]
    User,
    /// The rear-facing camera.
    Environment,
}

/// Whether a [`CameraInput`] takes a still or records a clip.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureMode {
    #[default]
    Image,
    Video,
}

/// A microphone recorder with a start/stop control.
#[derive(Clone, Serialize, Deserialize, Widget)]
pub struct AudioInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Replaces `label` while recording is in progress.
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_label: Option<String>,
    /// The container format to encode into. Defaults to `"audio/webm"`, the one
    /// format `MediaRecorder` supports everywhere.
    #[prop]
    pub mime_type: String,
    #[prop]
    pub disabled: bool,
    /// Draw a live level meter while recording. Rust-side only — Ivy's
    /// `AudioInputWidget` always shows its waveform and reads no such prop.
    #[prop]
    pub show_waveform: bool,
    /// POST the recording here instead of delivering it to `on_capture`.
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload_url: Option<String>,
    /// How often `MediaRecorder` emits a chunk, in milliseconds.
    #[prop]
    pub chunk_interval: u32,
    /// Requested capture rate in Hz. `None` leaves it to the device.
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Size>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid: Option<String>,
    #[prop]
    pub auto_focus: bool,
    /// Receives the finished recording as a base64 data URL.
    #[event(arg = "value")]
    #[serde(skip)]
    pub on_capture: Option<Arc<dyn Fn(String) + Send + Sync>>,
    #[event]
    #[serde(skip)]
    pub on_focus: Option<Arc<dyn Fn() + Send + Sync>>,
    #[event]
    #[serde(skip)]
    pub on_blur: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for AudioInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioInput")
            .field("label", &self.label)
            .field("mime_type", &self.mime_type)
            .field("disabled", &self.disabled)
            .finish()
    }
}

impl AudioInput {
    pub fn new() -> Self {
        AudioInput {
            id: None,
            label: None,
            recording_label: None,
            mime_type: "audio/webm".to_string(),
            disabled: false,
            show_waveform: true,
            upload_url: None,
            chunk_interval: 1000,
            sample_rate: None,
            width: None,
            invalid: None,
            auto_focus: false,
            on_capture: None,
            on_focus: None,
            on_blur: None,
        }
    }

    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn recording_label(mut self, label: &str) -> Self {
        self.recording_label = Some(label.to_string());
        self
    }

    pub fn mime_type(mut self, mime_type: &str) -> Self {
        self.mime_type = mime_type.to_string();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn show_waveform(mut self, show: bool) -> Self {
        self.show_waveform = show;
        self
    }

    pub fn upload_url(mut self, url: &str) -> Self {
        self.upload_url = Some(url.to_string());
        self
    }

    pub fn chunk_interval(mut self, interval: u32) -> Self {
        self.chunk_interval = interval;
        self
    }

    pub fn sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = Some(rate);
        self
    }

    pub fn width(mut self, width: Size) -> Self {
        self.width = Some(width);
        self
    }

    /// Mark the input invalid with a validation message.
    pub fn invalid(mut self, message: &str) -> Self {
        self.invalid = Some(message.to_string());
        self
    }

    pub fn auto_focus(mut self, auto_focus: bool) -> Self {
        self.auto_focus = auto_focus;
        self
    }

    pub fn on_capture(mut self, handler: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_capture = Some(Arc::new(handler));
        self
    }

    pub fn on_focus(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_focus = Some(Arc::new(handler));
        self
    }

    pub fn on_blur(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_blur = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for AudioInput {
    fn default() -> Self {
        Self::new()
    }
}

impl From<AudioInput> for Element {
    fn from(input: AudioInput) -> Self {
        input.into_element()
    }
}

/// A live camera preview with a shutter control.
#[derive(Clone, Serialize, Deserialize, Widget)]
pub struct CameraInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[prop]
    pub disabled: bool,
    /// POST the capture here instead of delivering it to `on_capture`.
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload_url: Option<String>,
    #[prop]
    pub facing_mode: FacingMode,
    /// Still or clip. Rust-side only — Ivy's `CameraInputWidget` always takes
    /// stills and reads no such prop.
    #[prop]
    pub capture_mode: CaptureMode,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Size>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid: Option<String>,
    #[prop]
    pub auto_focus: bool,
    /// Receives the capture as a base64 data URL.
    #[event(arg = "value")]
    #[serde(skip)]
    pub on_capture: Option<Arc<dyn Fn(String) + Send + Sync>>,
    #[event]
    #[serde(skip)]
    pub on_focus: Option<Arc<dyn Fn() + Send + Sync>>,
    #[event]
    #[serde(skip)]
    pub on_blur: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for CameraInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CameraInput")
            .field("facing_mode", &self.facing_mode)
            .field("capture_mode", &self.capture_mode)
            .field("disabled", &self.disabled)
            .finish()
    }
}

impl CameraInput {
    pub fn new() -> Self {
        CameraInput {
            id: None,
            placeholder: None,
            disabled: false,
            upload_url: None,
            facing_mode: FacingMode::User,
            capture_mode: CaptureMode::Image,
            width: None,
            invalid: None,
            auto_focus: false,
            on_capture: None,
            on_focus: None,
            on_blur: None,
        }
    }

    pub fn placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = Some(placeholder.to_string());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn upload_url(mut self, url: &str) -> Self {
        self.upload_url = Some(url.to_string());
        self
    }

    pub fn facing_mode(mut self, mode: FacingMode) -> Self {
        self.facing_mode = mode;
        self
    }

    pub fn capture_mode(mut self, mode: CaptureMode) -> Self {
        self.capture_mode = mode;
        self
    }

    pub fn width(mut self, width: Size) -> Self {
        self.width = Some(width);
        self
    }

    /// Mark the input invalid with a validation message.
    pub fn invalid(mut self, message: &str) -> Self {
        self.invalid = Some(message.to_string());
        self
    }

    pub fn auto_focus(mut self, auto_focus: bool) -> Self {
        self.auto_focus = auto_focus;
        self
    }

    pub fn on_capture(mut self, handler: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_capture = Some(Arc::new(handler));
        self
    }

    pub fn on_focus(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_focus = Some(Arc::new(handler));
        self
    }

    pub fn on_blur(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_blur = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for CameraInput {
    fn default() -> Self {
        Self::new()
    }
}

impl From<CameraInput> for Element {
    fn from(input: CameraInput) -> Self {
        input.into_element()
    }
}

/// A pad the user draws a signature on, producing a PNG.
#[derive(Clone, Serialize, Deserialize, Widget)]
pub struct SignatureInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The current signature as a base64-encoded PNG data URL.
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[prop]
    pub disabled: bool,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid: Option<String>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// Ink colour.
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pen: Option<Color>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Color>,
    /// Stroke width in CSS pixels.
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pen_thickness: Option<f64>,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub density: Option<Density>,
    #[prop]
    pub auto_focus: bool,
    /// Receives the finished signature as a base64 PNG data URL.
    #[event(arg = "value")]
    #[serde(skip)]
    pub on_sign: Option<Arc<dyn Fn(String) + Send + Sync>>,
    /// Fires when the pad is wiped. Ivy reports this as `OnChange` with a null
    /// value; the harness reports it as a distinct `clear` event.
    #[event]
    #[serde(skip)]
    pub on_clear: Option<Arc<dyn Fn() + Send + Sync>>,
    #[event]
    #[serde(skip)]
    pub on_focus: Option<Arc<dyn Fn() + Send + Sync>>,
    #[event]
    #[serde(skip)]
    pub on_blur: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for SignatureInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignatureInput")
            .field("has_value", &self.value.is_some())
            .field("disabled", &self.disabled)
            .finish()
    }
}

impl SignatureInput {
    pub fn new() -> Self {
        SignatureInput {
            id: None,
            value: None,
            disabled: false,
            invalid: None,
            placeholder: None,
            pen: None,
            background: None,
            pen_thickness: None,
            density: None,
            auto_focus: false,
            on_sign: None,
            on_clear: None,
            on_focus: None,
            on_blur: None,
        }
    }

    pub fn value(mut self, value: &str) -> Self {
        self.value = Some(value.to_string());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Mark the input invalid with a validation message.
    pub fn invalid(mut self, message: &str) -> Self {
        self.invalid = Some(message.to_string());
        self
    }

    pub fn placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = Some(placeholder.to_string());
        self
    }

    pub fn pen(mut self, pen: Color) -> Self {
        self.pen = Some(pen);
        self
    }

    pub fn background(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }

    pub fn pen_thickness(mut self, thickness: f64) -> Self {
        self.pen_thickness = Some(thickness);
        self
    }

    pub fn density(mut self, density: Density) -> Self {
        self.density = Some(density);
        self
    }

    pub fn auto_focus(mut self, auto_focus: bool) -> Self {
        self.auto_focus = auto_focus;
        self
    }

    pub fn on_sign(mut self, handler: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_sign = Some(Arc::new(handler));
        self
    }

    pub fn on_clear(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_clear = Some(Arc::new(handler));
        self
    }

    pub fn on_focus(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_focus = Some(Arc::new(handler));
        self
    }

    pub fn on_blur(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_blur = Some(Arc::new(handler));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for SignatureInput {
    fn default() -> Self {
        Self::new()
    }
}

impl From<SignatureInput> for Element {
    fn from(input: SignatureInput) -> Self {
        input.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::shared::NamedColor;
    use crate::views::view::{BuildContext, WidgetData};
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    #[test]
    fn test_audio_input_builder_round_trip() {
        let input = AudioInput::new()
            .label("Record")
            .recording_label("Stop")
            .mime_type("audio/ogg")
            .disabled(true)
            .show_waveform(false)
            .upload_url("/uploads")
            .chunk_interval(250)
            .sample_rate(44100)
            .width(Size::Px(320.0))
            .invalid("Recording required")
            .auto_focus(true);

        assert_eq!(input.label.as_deref(), Some("Record"));
        assert_eq!(input.recording_label.as_deref(), Some("Stop"));
        assert_eq!(input.mime_type, "audio/ogg");
        assert!(input.disabled);
        assert!(!input.show_waveform);
        assert_eq!(input.upload_url.as_deref(), Some("/uploads"));
        assert_eq!(input.chunk_interval, 250);
        assert_eq!(input.sample_rate, Some(44100));
        assert_eq!(input.width, Some(Size::Px(320.0)));
        assert_eq!(input.invalid.as_deref(), Some("Recording required"));
        assert!(input.auto_focus);
    }

    #[test]
    fn test_audio_input_defaults() {
        let input = AudioInput::default();
        assert!(input.label.is_none());
        assert_eq!(input.mime_type, "audio/webm");
        assert!(!input.disabled);
        assert!(input.show_waveform);
        assert!(input.upload_url.is_none());
        assert_eq!(input.chunk_interval, 1000);
        assert!(input.sample_rate.is_none());
        assert!(!input.auto_focus);
    }

    #[test]
    fn test_audio_input_to_json_keys() {
        let json = AudioInput::new()
            .label("Record")
            .recording_label("Stop")
            .sample_rate(48000)
            .on_capture(|_| {})
            .on_focus(|| {})
            .on_blur(|| {})
            .to_json();

        assert_eq!(json["type"], "audio_input");
        assert_eq!(json["label"], "Record");
        assert_eq!(json["recordingLabel"], "Stop");
        assert_eq!(json["mimeType"], "audio/webm");
        assert_eq!(json["showWaveform"], true);
        assert_eq!(json["chunkInterval"], 1000);
        assert_eq!(json["sampleRate"], 48000);
        assert_eq!(json["autoFocus"], false);
        assert_eq!(json["hasOnCapture"], true);
        assert_eq!(json["hasOnFocus"], true);
        assert_eq!(json["hasOnBlur"], true);
    }

    #[test]
    fn test_audio_input_json_without_handlers() {
        let json = AudioInput::new().to_json();
        assert_eq!(json["hasOnCapture"], false);
        assert_eq!(json["hasOnFocus"], false);
        assert_eq!(json["hasOnBlur"], false);
    }

    #[test]
    fn test_audio_input_assign_ids() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);
        let mut element: Element = AudioInput::new().into();
        element.assign_ids(&mut ctx);
        if let Element::Widget(ref w) = element {
            assert_eq!(w.get_id(), Some("w-0"));
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_audio_input_capture_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received = Arc::new(Mutex::new(None::<String>));
        let received_clone = received.clone();
        let mut element: Element = AudioInput::new()
            .on_capture(move |value| {
                *received_clone.lock().unwrap() = Some(value);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch(
            "w-0",
            "capture",
            json!({"value": "data:audio/webm;base64,AAA"})
        ));
        assert_eq!(
            received.lock().unwrap().as_deref(),
            Some("data:audio/webm;base64,AAA")
        );
    }

    #[test]
    fn test_audio_input_focus_and_blur_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let focus_count = Arc::new(AtomicUsize::new(0));
        let blur_count = Arc::new(AtomicUsize::new(0));
        let focus_clone = focus_count.clone();
        let blur_clone = blur_count.clone();

        let mut element: Element = AudioInput::new()
            .on_focus(move || {
                focus_clone.fetch_add(1, Ordering::SeqCst);
            })
            .on_blur(move || {
                blur_clone.fetch_add(1, Ordering::SeqCst);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "focus", serde_json::Value::Null));
        assert!(registry.dispatch("w-0", "blur", serde_json::Value::Null));
        assert_eq!(focus_count.load(Ordering::SeqCst), 1);
        assert_eq!(blur_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_camera_input_builder_round_trip() {
        let input = CameraInput::new()
            .placeholder("Smile")
            .disabled(true)
            .upload_url("/uploads")
            .facing_mode(FacingMode::Environment)
            .capture_mode(CaptureMode::Video)
            .width(Size::Percent(100.0))
            .invalid("Photo required")
            .auto_focus(true);

        assert_eq!(input.placeholder.as_deref(), Some("Smile"));
        assert!(input.disabled);
        assert_eq!(input.upload_url.as_deref(), Some("/uploads"));
        assert_eq!(input.facing_mode, FacingMode::Environment);
        assert_eq!(input.capture_mode, CaptureMode::Video);
        assert_eq!(input.width, Some(Size::Percent(100.0)));
        assert_eq!(input.invalid.as_deref(), Some("Photo required"));
        assert!(input.auto_focus);
    }

    #[test]
    fn test_camera_input_defaults() {
        let input = CameraInput::default();
        assert!(input.placeholder.is_none());
        assert!(!input.disabled);
        assert!(input.upload_url.is_none());
        assert_eq!(input.facing_mode, FacingMode::User);
        assert_eq!(input.capture_mode, CaptureMode::Image);
        assert!(!input.auto_focus);
    }

    #[test]
    fn test_camera_input_to_json_keys() {
        let json = CameraInput::new()
            .placeholder("Smile")
            .upload_url("/uploads")
            .facing_mode(FacingMode::Environment)
            .capture_mode(CaptureMode::Video)
            .on_capture(|_| {})
            .to_json();

        assert_eq!(json["type"], "camera_input");
        assert_eq!(json["placeholder"], "Smile");
        assert_eq!(json["uploadUrl"], "/uploads");
        assert_eq!(json["facingMode"], "environment");
        assert_eq!(json["captureMode"], "video");
        assert_eq!(json["hasOnCapture"], true);
        assert_eq!(json["hasOnFocus"], false);
        assert_eq!(json["hasOnBlur"], false);
    }

    #[test]
    fn test_camera_input_capture_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received = Arc::new(Mutex::new(None::<String>));
        let received_clone = received.clone();
        let mut element: Element = CameraInput::new()
            .on_capture(move |value| {
                *received_clone.lock().unwrap() = Some(value);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch(
            "w-0",
            "capture",
            json!({"value": "data:image/png;base64,BBB"})
        ));
        assert_eq!(
            received.lock().unwrap().as_deref(),
            Some("data:image/png;base64,BBB")
        );
    }

    #[test]
    fn test_camera_input_focus_and_blur_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let focus_count = Arc::new(AtomicUsize::new(0));
        let blur_count = Arc::new(AtomicUsize::new(0));
        let focus_clone = focus_count.clone();
        let blur_clone = blur_count.clone();

        let mut element: Element = CameraInput::new()
            .on_focus(move || {
                focus_clone.fetch_add(1, Ordering::SeqCst);
            })
            .on_blur(move || {
                blur_clone.fetch_add(1, Ordering::SeqCst);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "focus", serde_json::Value::Null));
        assert!(registry.dispatch("w-0", "blur", serde_json::Value::Null));
        assert_eq!(focus_count.load(Ordering::SeqCst), 1);
        assert_eq!(blur_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_facing_and_capture_modes_serialize_lower_case() {
        assert_eq!(
            serde_json::to_string(&FacingMode::User).unwrap(),
            "\"user\""
        );
        assert_eq!(
            serde_json::to_string(&FacingMode::Environment).unwrap(),
            "\"environment\""
        );
        assert_eq!(
            serde_json::to_string(&CaptureMode::Image).unwrap(),
            "\"image\""
        );
        assert_eq!(
            serde_json::to_string(&CaptureMode::Video).unwrap(),
            "\"video\""
        );
    }

    #[test]
    fn test_signature_input_builder_round_trip() {
        let input = SignatureInput::new()
            .value("data:image/png;base64,CCC")
            .disabled(true)
            .invalid("Signature required")
            .placeholder("Sign here")
            .pen(Color::Named(NamedColor::Primary))
            .background(Color::Named(NamedColor::Muted))
            .pen_thickness(2.5)
            .density(Density::Comfortable)
            .auto_focus(true);

        assert_eq!(input.value.as_deref(), Some("data:image/png;base64,CCC"));
        assert!(input.disabled);
        assert_eq!(input.invalid.as_deref(), Some("Signature required"));
        assert_eq!(input.placeholder.as_deref(), Some("Sign here"));
        assert_eq!(input.pen, Some(Color::Named(NamedColor::Primary)));
        assert_eq!(input.background, Some(Color::Named(NamedColor::Muted)));
        assert_eq!(input.pen_thickness, Some(2.5));
        assert_eq!(input.density, Some(Density::Comfortable));
        assert!(input.auto_focus);
    }

    #[test]
    fn test_signature_input_defaults() {
        let input = SignatureInput::default();
        assert!(input.value.is_none());
        assert!(!input.disabled);
        assert!(input.invalid.is_none());
        assert!(input.pen.is_none());
        assert!(input.background.is_none());
        assert!(input.pen_thickness.is_none());
        assert!(input.density.is_none());
        assert!(!input.auto_focus);
    }

    #[test]
    fn test_signature_input_to_json_keys() {
        let json = SignatureInput::new()
            .value("data:image/png;base64,CCC")
            .pen_thickness(3.0)
            .placeholder("Sign here")
            .on_sign(|_| {})
            .on_clear(|| {})
            .to_json();

        assert_eq!(json["type"], "signature_input");
        assert_eq!(json["value"], "data:image/png;base64,CCC");
        assert_eq!(json["penThickness"], 3.0);
        assert_eq!(json["placeholder"], "Sign here");
        assert_eq!(json["hasOnSign"], true);
        assert_eq!(json["hasOnClear"], true);
        assert_eq!(json["hasOnFocus"], false);
        assert_eq!(json["hasOnBlur"], false);
    }

    #[test]
    fn test_signature_input_sign_and_clear_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let received = Arc::new(Mutex::new(None::<String>));
        let received_clone = received.clone();
        let clears = Arc::new(AtomicUsize::new(0));
        let clears_clone = clears.clone();

        let mut element: Element = SignatureInput::new()
            .on_sign(move |value| {
                *received_clone.lock().unwrap() = Some(value);
            })
            .on_clear(move || {
                clears_clone.fetch_add(1, Ordering::SeqCst);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "sign", json!({"value": "data:image/png;base64,DDD"})));
        assert!(registry.dispatch("w-0", "clear", serde_json::Value::Null));
        assert_eq!(
            received.lock().unwrap().as_deref(),
            Some("data:image/png;base64,DDD")
        );
        assert_eq!(clears.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_signature_input_focus_and_blur_dispatch() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let focus_count = Arc::new(AtomicUsize::new(0));
        let blur_count = Arc::new(AtomicUsize::new(0));
        let focus_clone = focus_count.clone();
        let blur_clone = blur_count.clone();

        let mut element: Element = SignatureInput::new()
            .on_focus(move || {
                focus_clone.fetch_add(1, Ordering::SeqCst);
            })
            .on_blur(move || {
                blur_clone.fetch_add(1, Ordering::SeqCst);
            })
            .into();
        element.assign_ids(&mut ctx);

        let registry = ctx.take_event_registry();
        assert!(registry.dispatch("w-0", "focus", serde_json::Value::Null));
        assert!(registry.dispatch("w-0", "blur", serde_json::Value::Null));
        assert_eq!(focus_count.load(Ordering::SeqCst), 1);
        assert_eq!(blur_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_media_inputs_into_element() {
        assert!(matches!(
            Element::from(AudioInput::new()),
            Element::Widget(_)
        ));
        assert!(matches!(
            Element::from(CameraInput::new()),
            Element::Widget(_)
        ));
        assert!(matches!(
            Element::from(SignatureInput::new()),
            Element::Widget(_)
        ));
    }
}
