use serde::{Deserialize, Serialize};
use simdnbt::{ToNbtTag, owned::NbtCompound};

use crate::text::color::{ARGBColor, Color};

/// The style of a text component.
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct Style {
    /// The color to render the content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// Whether to render the content in bold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    /// Whether to render the content in italic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    /// Whether to render the content in underlined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlined: Option<bool>,
    /// Whether to render the content in strikethrough.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<bool>,
    /// Whether to render the content in obfuscated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obfuscated: Option<bool>,
    /// When the text is shift-clicked by a player, this string is inserted in their chat input. It does not overwrite any existing text the player was writing. This only works in chat messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insertion: Option<String>,
    /// Allows you to change the font of the text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "shadow_color"
    )]
    /// The color of the shadow.
    pub shadow_color: Option<ARGBColor>,
}

impl Style {
    /// Creates a new `Style`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            color: None,
            bold: None,
            italic: None,
            underlined: None,
            strikethrough: None,
            obfuscated: None,
            insertion: None,
            font: None,
            shadow_color: None,
        }
    }

    /// Sets the color of the `Style`.
    #[must_use]
    pub const fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets the shadow color of the `Style`.
    #[must_use]
    pub const fn shadow_color(mut self, shadow_color: ARGBColor) -> Self {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Converts the `Style` into an `NbtCompound`.
    #[must_use]
    pub fn into_nbt_compound(self) -> NbtCompound {
        let mut compound = NbtCompound::new();
        if let Some(color) = self.color {
            let color = match color {
                Color::Named(color) => Some(color.to_string()),
                Color::Rgb(color) => Some(format!(
                    "#{:02X}{:02X}{:02X}",
                    color.red, color.green, color.blue
                )),
                //TODO: A reset should reset the whole style
                Color::Reset => None,
            };
            if let Some(color) = color {
                compound.insert("color", color);
            }
        }

        if let Some(bold) = self.bold {
            compound.insert("bold", bold);
        }

        if let Some(italic) = self.italic {
            compound.insert("italic", italic);
        }

        if let Some(underlined) = self.underlined {
            compound.insert("underlined", underlined);
        }

        if let Some(strikethrough) = self.strikethrough {
            compound.insert("strikethrough", strikethrough);
        }

        if let Some(obfuscated) = self.obfuscated {
            compound.insert("obfuscated", obfuscated);
        }

        if let Some(insertion) = self.insertion {
            compound.insert("insertion", insertion);
        }

        if let Some(font) = self.font {
            compound.insert("font", font);
        }

        if let Some(shadow_color) = self.shadow_color {
            compound.insert("shadow_color", shadow_color.to_nbt_tag());
        }

        compound
    }
}
