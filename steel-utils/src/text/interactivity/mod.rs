/// A module for click events.
pub mod click;
/// A module for hover events.
pub mod hover;

pub use click::ClickEvent;
pub use hover::HoverEvent;

use serde::{Deserialize, Serialize};

/// The interactivity of a text component.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Interactivity {
    /// Allows for events to occur when the player clicks on text. Only works in chat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub click_event: Option<ClickEvent>,
    /// Allows for a tooltip to be displayed when the player hovers their mouse over text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_event: Option<HoverEvent>,
}

impl Interactivity {
    /// Creates a new `Interactivity`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            click_event: None,
            hover_event: None,
        }
    }

    /// Sets the click event for the `Interactivity`.
    #[must_use]
    pub fn click_event(mut self, click_event: ClickEvent) -> Self {
        self.click_event = Some(click_event);
        self
    }

    /// Sets the hover event for the `Interactivity`.
    #[must_use]
    pub fn hover_event(mut self, hover_event: HoverEvent) -> Self {
        self.hover_event = Some(hover_event);
        self
    }
}

#[allow(missing_docs)]
impl Default for Interactivity {
    fn default() -> Self {
        Self::new()
    }
}
