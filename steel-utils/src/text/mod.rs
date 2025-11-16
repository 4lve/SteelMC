//! This module contains everything related to text components.
use serde::{Deserialize, Serialize};
use simdnbt::{
    ToNbtTag,
    owned::{NbtCompound, NbtList, NbtTag},
};
use std::{
    borrow::Cow,
    fmt::{self, Display},
};

use crate::text::{
    color::Color, interactivity::Interactivity, style::Style, translation::TranslatedMessage,
};

/// A module for click events.
pub mod click;
/// A module for colors.
pub mod color;
/// A module for hover events.
pub mod hover;
/// A module for interactivity.
pub mod interactivity;
/// A module for locales.
pub mod locale;
/// A module for styles.
pub mod style;
/// A module for translations.
pub mod translation;

/// A text component.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TextComponent {
    /// The actual text
    #[serde(flatten)]
    pub content: TextContent,
    /// Style of the text. Bold, Italic, underline, Color...
    #[serde(flatten)]
    pub style: Style,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Extra text components
    pub extra: Vec<TextComponent>,
    /// Interactivity of the text. Click event, hover event, etc.
    #[serde(flatten)]
    pub interactivity: Interactivity,
}

/// The content of a text component.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum TextContent {
    /// Raw text
    Text {
        /// The text.
        text: Cow<'static, str>,
    },
    /// Translated text
    Translate(TranslatedMessage),
    /// A keybind identifier
    /// <https://minecraft.wiki/w/Controls#Configurable_controls>
    Keybind {
        /// The keybind.
        keybind: Cow<'static, str>,
    },
}

#[allow(missing_docs)]
impl From<String> for TextComponent {
    fn from(value: String) -> Self {
        Self::new().text(value)
    }
}

#[allow(missing_docs)]
impl From<&'static str> for TextComponent {
    fn from(value: &'static str) -> Self {
        Self::new().text(value)
    }
}

#[allow(missing_docs)]
impl Default for TextComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl TextComponent {
    /// Creates a new text component.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            content: TextContent::Text {
                text: Cow::Borrowed(""),
            },
            style: Style::new(),
            extra: Vec::new(),
            interactivity: Interactivity::new(),
        }
    }

    /// Sets the text component to be a translated message.
    #[must_use]
    pub fn translate(mut self, translation: TranslatedMessage) -> Self {
        self.content = TextContent::Translate(translation);
        self
    }

    /// Sets the text component to be raw text.
    #[must_use]
    pub fn text(mut self, text: impl Into<Cow<'static, str>>) -> Self {
        self.content = TextContent::Text { text: text.into() };
        self
    }

    /// Sets the color of the text component.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.style = self.style.color(color);
        self
    }

    /// Sets the style of the text component.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Adds an extra text component to this text component.
    #[must_use]
    pub fn extra(mut self, extra: impl Into<TextComponent>) -> Self {
        self.extra.push(extra.into());
        self
    }

    /// Sets the interactivity of the text component.
    #[must_use]
    pub fn interactivity(mut self, interactivity: Interactivity) -> Self {
        self.interactivity = interactivity;
        self
    }

    /// Creates a new text component with the given text.
    #[must_use]
    pub const fn const_text(text: &'static str) -> Self {
        Self {
            content: TextContent::Text {
                text: Cow::Borrowed(text),
            },
            style: Style::new(),
            extra: Vec::new(),
            interactivity: Interactivity::new(),
        }
    }

    /// Creates a new text component with the given text and color.
    #[must_use]
    pub const fn text_with_color(text: Cow<'static, str>, color: Color) -> Self {
        Self {
            content: TextContent::Text { text },
            style: Style::new().color(color),
            extra: Vec::new(),
            interactivity: Interactivity::new(),
        }
    }

    /// Creates a new text component with the given translation key.
    #[must_use]
    pub const fn const_translate(key: &'static str) -> Self {
        Self {
            content: TextContent::Translate(TranslatedMessage::new(key, None)),
            style: Style::new(),
            extra: Vec::new(),
            interactivity: Interactivity::new(),
        }
    }

    /// Creates a new text component with the given translation key and color.
    #[must_use]
    pub const fn const_translate_with_color(key: &'static str, color: Color) -> Self {
        Self {
            content: TextContent::Translate(TranslatedMessage::new(key, None)),
            style: Style::new().color(color),
            extra: Vec::new(),
            interactivity: Interactivity::new(),
        }
    }

    /// Converts the text component into an NBT compound.
    #[must_use]
    pub fn into_nbt_compound(self) -> NbtCompound {
        let mut compound = NbtCompound::new();
        match self.content {
            TextContent::Text { text } => compound.insert("text", text.to_string()),
            TextContent::Translate(message) => {
                compound.insert("translate", message.key.to_string());
                if let Some(fallback) = message.fallback {
                    compound.insert("fallback", fallback.to_string());
                }
                if let Some(args) = message.args {
                    compound.insert(
                        "with",
                        NbtTag::List(NbtList::Compound(
                            args.into_iter()
                                .map(TextComponent::into_nbt_compound)
                                .collect(),
                        )),
                    );
                }
            }
            TextContent::Keybind { keybind } => compound.insert("keybind", keybind.to_string()),
        }

        compound.extend(self.style.into_nbt_compound());

        if !self.extra.is_empty() {
            compound.insert(
                "extra",
                NbtList::Compound(
                    self.extra
                        .into_iter()
                        .map(TextComponent::into_nbt_compound)
                        .collect(),
                ),
            );
        }

        log::info!("compound: {compound:?}");
        compound
    }
}

#[allow(missing_docs)]
impl ToNbtTag for TextComponent {
    fn to_nbt_tag(self) -> NbtTag {
        NbtTag::Compound(self.into_nbt_compound())
    }
}

#[allow(missing_docs)]
impl Display for TextComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.content {
            TextContent::Text { text } => write!(f, "{text}"),
            TextContent::Translate(message) => write!(f, "{}", message.format()),
            TextContent::Keybind { keybind } => write!(f, "{keybind}"),
        }
    }
}
