use crate::widgets::button::ButtonVariant;

/// What the user chose in an alert.
///
/// Ported from Ivy-Framework's `Views/Alerts/AlertOptions.cs`. `Undecided` is the
/// state before any button is clicked, so a callback can tell "dismissed" apart
/// from an explicit answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlertResult {
    #[default]
    Undecided,
    Ok,
    Cancel,
    Yes,
    No,
}

impl AlertResult {
    /// Ivy's `AlertResultExtensions.IsOk`.
    pub fn is_ok(&self) -> bool {
        *self == AlertResult::Ok
    }
}

/// Which buttons an alert offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlertButtonSet {
    #[default]
    Ok,
    OkCancel,
    YesNo,
    YesNoCancel,
}

/// One button in an alert: its label, the result it produces, and how it looks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertButton {
    pub label: String,
    pub result: AlertResult,
    pub variant: ButtonVariant,
}

impl AlertButton {
    pub fn new(label: impl Into<String>, result: AlertResult, variant: ButtonVariant) -> Self {
        AlertButton {
            label: label.into(),
            result,
            variant,
        }
    }

    /// A primary button, the default in Ivy's `AlertButton` constructor.
    pub fn primary(label: impl Into<String>, result: AlertResult) -> Self {
        AlertButton::new(label, result, ButtonVariant::Primary)
    }
}

/// The title, message and buttons of one alert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertOptions {
    pub title: Option<String>,
    pub message: Option<String>,
    pub buttons: Vec<AlertButton>,
}

impl AlertOptions {
    pub fn new(title: Option<String>, message: Option<String>, button_set: AlertButtonSet) -> Self {
        AlertOptions {
            title,
            message,
            buttons: buttons_for(button_set),
        }
    }
}

/// The buttons for a set, in Ivy's order — the dismissing choice first, so the
/// affirmative one lands closest to the right edge of the footer.
pub fn buttons_for(button_set: AlertButtonSet) -> Vec<AlertButton> {
    match button_set {
        AlertButtonSet::Ok => vec![AlertButton::primary("Ok", AlertResult::Ok)],
        AlertButtonSet::OkCancel => vec![
            AlertButton::new("Cancel", AlertResult::Cancel, ButtonVariant::Secondary),
            AlertButton::primary("Ok", AlertResult::Ok),
        ],
        AlertButtonSet::YesNo => vec![
            AlertButton::new("No", AlertResult::No, ButtonVariant::Secondary),
            AlertButton::primary("Yes", AlertResult::Yes),
        ],
        AlertButtonSet::YesNoCancel => vec![
            AlertButton::new("Cancel", AlertResult::Cancel, ButtonVariant::Secondary),
            AlertButton::primary("No", AlertResult::No),
            AlertButton::primary("Yes", AlertResult::Yes),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ok_only_for_ok() {
        assert!(AlertResult::Ok.is_ok());
        for result in [
            AlertResult::Undecided,
            AlertResult::Cancel,
            AlertResult::Yes,
            AlertResult::No,
        ] {
            assert!(!result.is_ok(), "{result:?} must not be Ok");
        }
    }

    #[test]
    fn test_button_sets_match_ivy() {
        let labels = |set| {
            buttons_for(set)
                .into_iter()
                .map(|b| (b.label, b.result, b.variant))
                .collect::<Vec<_>>()
        };

        assert_eq!(
            labels(AlertButtonSet::Ok),
            vec![("Ok".to_string(), AlertResult::Ok, ButtonVariant::Primary)]
        );
        assert_eq!(
            labels(AlertButtonSet::OkCancel),
            vec![
                (
                    "Cancel".to_string(),
                    AlertResult::Cancel,
                    ButtonVariant::Secondary
                ),
                ("Ok".to_string(), AlertResult::Ok, ButtonVariant::Primary),
            ]
        );
        assert_eq!(
            labels(AlertButtonSet::YesNo),
            vec![
                ("No".to_string(), AlertResult::No, ButtonVariant::Secondary),
                ("Yes".to_string(), AlertResult::Yes, ButtonVariant::Primary),
            ]
        );
        assert_eq!(
            labels(AlertButtonSet::YesNoCancel),
            vec![
                (
                    "Cancel".to_string(),
                    AlertResult::Cancel,
                    ButtonVariant::Secondary
                ),
                ("No".to_string(), AlertResult::No, ButtonVariant::Primary),
                ("Yes".to_string(), AlertResult::Yes, ButtonVariant::Primary),
            ]
        );
    }

    #[test]
    fn test_alert_options_carries_its_button_set() {
        let options = AlertOptions::new(
            Some("Delete?".to_string()),
            Some("This cannot be undone.".to_string()),
            AlertButtonSet::YesNo,
        );

        assert_eq!(options.title.as_deref(), Some("Delete?"));
        assert_eq!(options.message.as_deref(), Some("This cannot be undone."));
        assert_eq!(options.buttons, buttons_for(AlertButtonSet::YesNo));
    }
}
