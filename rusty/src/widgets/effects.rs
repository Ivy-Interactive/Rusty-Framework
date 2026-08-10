use crate::views::view::{BuildContext, Element};
use rusty_macros::Widget;
use serde::{Deserialize, Serialize};

/// What causes an effect to play.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EffectTrigger {
    #[default]
    Auto,
    Click,
    Hover,
}

/// A one-shot celebration effect (confetti) wrapping its children.
#[derive(Debug, Clone, Serialize, Deserialize, Widget)]
pub struct Confetti {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    pub trigger: EffectTrigger,
    #[prop]
    #[children]
    pub children: Vec<Element>,
}

impl Confetti {
    pub fn new() -> Self {
        Confetti {
            id: None,
            trigger: EffectTrigger::default(),
            children: Vec::new(),
        }
    }

    pub fn trigger(mut self, trigger: EffectTrigger) -> Self {
        self.trigger = trigger;
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

impl Default for Confetti {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Confetti> for Element {
    fn from(confetti: Confetti) -> Self {
        confetti.into_element()
    }
}

/// The kind of motion an [`Animation`] plays.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimationType {
    #[default]
    Rotate,
    SlideIn,
    FadeIn,
    ZoomIn,
    SlideOut,
    FadeOut,
    ZoomOut,
    Bounce,
    Shake,
    Flip,
    Stagger,
    Wave,
    Pulse,
    Spring,
    Hover,
}

/// The easing curve applied to an [`Animation`]'s transition.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimationEasing {
    EaseIn,
    EaseOut,
    EaseInOut,
    #[default]
    Linear,
    CircIn,
    CircOut,
    CircInOut,
    BackIn,
    BackOut,
    BackInOut,
    Anticipate,
    AnticipateOut,
    BounceIn,
    BounceOut,
    BounceInOut,
    ElasticIn,
    ElasticOut,
    ElasticInOut,
}

/// The direction an [`Animation`] slides in from or out to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimationDirection {
    Left,
    Right,
    Up,
    Down,
}

/// A Lottie/CSS animation wrapper around its children.
///
/// The kind of animation is named `animation_type` rather than `type`: the
/// `Widget` derive already writes the widget discriminator under the JSON key
/// `"type"`, so a prop literally named `type` would overwrite it.
#[derive(Debug, Clone, Serialize, Deserialize, Widget)]
pub struct Animation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[prop]
    pub animation_type: AnimationType,
    #[prop]
    pub duration: f64,
    #[prop]
    pub delay: f64,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<AnimationDirection>,
    #[prop]
    pub distance: f64,
    #[prop]
    pub easing: AnimationEasing,
    #[prop]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat: Option<i32>,
    #[prop]
    pub repeat_delay: f64,
    #[prop]
    pub visible: bool,
    #[prop]
    pub intensity: f64,
    #[prop]
    pub trigger: EffectTrigger,
    #[prop]
    #[children]
    pub children: Vec<Element>,
}

impl Animation {
    pub fn new() -> Self {
        Animation {
            id: None,
            animation_type: AnimationType::default(),
            duration: 0.5,
            delay: 0.0,
            direction: None,
            distance: 100.0,
            easing: AnimationEasing::default(),
            repeat: None,
            repeat_delay: 0.0,
            visible: true,
            intensity: 1.0,
            trigger: EffectTrigger::default(),
            children: Vec::new(),
        }
    }

    pub fn animation_type(mut self, animation_type: AnimationType) -> Self {
        self.animation_type = animation_type;
        self
    }

    pub fn duration(mut self, duration: f64) -> Self {
        self.duration = duration;
        self
    }

    pub fn delay(mut self, delay: f64) -> Self {
        self.delay = delay;
        self
    }

    pub fn direction(mut self, direction: AnimationDirection) -> Self {
        self.direction = Some(direction);
        self
    }

    pub fn distance(mut self, distance: f64) -> Self {
        self.distance = distance;
        self
    }

    pub fn easing(mut self, easing: AnimationEasing) -> Self {
        self.easing = easing;
        self
    }

    pub fn repeat(mut self, repeat: i32) -> Self {
        self.repeat = Some(repeat);
        self
    }

    pub fn repeat_delay(mut self, repeat_delay: f64) -> Self {
        self.repeat_delay = repeat_delay;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn intensity(mut self, intensity: f64) -> Self {
        self.intensity = intensity;
        self
    }

    pub fn trigger(mut self, trigger: EffectTrigger) -> Self {
        self.trigger = trigger;
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

    /// Assign a widget ID from the BuildContext.
    #[deprecated(note = "Widget IDs are now assigned automatically. Remove .build(ctx) calls.")]
    pub fn build(mut self, ctx: &mut BuildContext) -> Self {
        self.id = Some(ctx.next_widget_id());
        self
    }

    pub fn into_element(self) -> Element {
        Element::Widget(Box::new(self))
    }
}

impl Default for Animation {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Animation> for Element {
    fn from(animation: Animation) -> Self {
        animation.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::hook_store::HookStore;
    use crate::views::view::WidgetData;
    use crate::widgets::text::TextBlock;

    #[test]
    fn test_confetti_builder() {
        let confetti = Confetti::new()
            .trigger(EffectTrigger::Click)
            .child(TextBlock::new("Party"));

        assert_eq!(confetti.trigger, EffectTrigger::Click);
        assert_eq!(confetti.children.len(), 1);
    }

    #[test]
    fn test_confetti_json() {
        let json = Confetti::new()
            .trigger(EffectTrigger::Hover)
            .child(TextBlock::new("Party"))
            .to_json();

        assert_eq!(json["type"], "confetti");
        assert_eq!(json["trigger"], "hover");
        assert_eq!(json["children"][0]["content"], "Party");
    }

    #[test]
    fn test_confetti_default_trigger_is_auto() {
        assert_eq!(Confetti::new().to_json()["trigger"], "auto");
    }

    #[test]
    fn test_confetti_assign_ids_recurses_into_children() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = Confetti::new()
            .child(TextBlock::new("One"))
            .child(TextBlock::new("Two"))
            .into();

        element.assign_ids(&mut ctx);

        if let Element::Widget(ref w) = element {
            let json = w.to_json();
            assert_eq!(json["id"], "w-0");
            assert_eq!(json["children"][0]["id"], "w-1");
            assert_eq!(json["children"][1]["id"], "w-2");
        } else {
            panic!("Expected Element::Widget");
        }
    }

    #[test]
    fn test_animation_builder() {
        let animation = Animation::new()
            .animation_type(AnimationType::Bounce)
            .duration(1.5)
            .direction(AnimationDirection::Up)
            .easing(AnimationEasing::EaseInOut)
            .repeat(3)
            .visible(false)
            .child(TextBlock::new("Bouncy"));

        assert_eq!(animation.animation_type, AnimationType::Bounce);
        assert_eq!(animation.duration, 1.5);
        assert_eq!(animation.direction, Some(AnimationDirection::Up));
        assert_eq!(animation.easing, AnimationEasing::EaseInOut);
        assert_eq!(animation.repeat, Some(3));
        assert!(!animation.visible);
        assert_eq!(animation.children.len(), 1);
    }

    #[test]
    fn test_animation_json_uses_animation_type_key_not_type() {
        let json = Animation::new()
            .animation_type(AnimationType::SlideIn)
            .easing(AnimationEasing::BackOut)
            .repeat_delay(0.2)
            .to_json();

        // The widget discriminator, not the animation kind.
        assert_eq!(json["type"], "animation");
        assert_eq!(json["animationType"], "slideIn");
        assert_eq!(json["easing"], "backOut");
        assert_eq!(json["repeatDelay"], 0.2);
    }

    #[test]
    fn test_animation_defaults_serialize() {
        let json = Animation::new().to_json();
        assert_eq!(json["visible"], true);
        assert_eq!(json["duration"], 0.5);
        assert_eq!(json["delay"], 0.0);
        assert_eq!(json["distance"], 100.0);
        assert_eq!(json["intensity"], 1.0);
        assert_eq!(json["animationType"], "rotate");
        assert_eq!(json["easing"], "linear");
        assert_eq!(json["trigger"], "auto");
        assert!(json["repeat"].is_null());
        assert!(json["direction"].is_null());
    }

    #[test]
    fn test_animation_assign_ids_recurses_into_children() {
        let mut store = HookStore::default();
        let mut ctx = BuildContext::new(&mut store, None);

        let mut element: Element = Animation::new().child(TextBlock::new("Child")).into();

        element.assign_ids(&mut ctx);

        if let Element::Widget(ref w) = element {
            let json = w.to_json();
            assert_eq!(json["id"], "w-0");
            assert_eq!(json["children"][0]["id"], "w-1");
        } else {
            panic!("Expected Element::Widget");
        }
    }
}
