use serde::{Deserialize, Serialize};
use simdnbt::{ToNbtTag, owned::NbtCompound};

use crate::{
    hash::{ComponentHasher, HashEntry},
    text::color::{ARGBColor, Color},
};

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

    /// Returns true if this style has no properties set.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.color.is_none()
            && self.bold.is_none()
            && self.italic.is_none()
            && self.underlined.is_none()
            && self.strikethrough.is_none()
            && self.obfuscated.is_none()
            && self.insertion.is_none()
            && self.font.is_none()
            && self.shadow_color.is_none()
    }

    /// Hash the style fields into the provided entries list for map hashing.
    /// Field names match Minecraft's `Style.Serializer.MAP_CODEC`.
    pub fn hash_fields(&self, entries: &mut Vec<HashEntry>) {
        // color
        if let Some(color) = &self.color {
            let mut key_hasher = ComponentHasher::new();
            key_hasher.put_string("color");
            let mut value_hasher = ComponentHasher::new();
            // Color is encoded as a string in the CODEC
            let color_str = match color {
                Color::Named(named) => named.to_string(),
                Color::Rgb(rgb) => format!("#{:02X}{:02X}{:02X}", rgb.red, rgb.green, rgb.blue),
                Color::Reset => return, // Reset is not serialized
            };
            value_hasher.put_string(&color_str);
            entries.push(HashEntry::new(key_hasher, value_hasher));
        }

        // shadow_color (as int)
        if let Some(shadow_color) = &self.shadow_color {
            let mut key_hasher = ComponentHasher::new();
            key_hasher.put_string("shadow_color");
            let mut value_hasher = ComponentHasher::new();
            value_hasher.put_int(shadow_color.to_argb());
            entries.push(HashEntry::new(key_hasher, value_hasher));
        }

        // bold
        if let Some(bold) = self.bold {
            let mut key_hasher = ComponentHasher::new();
            key_hasher.put_string("bold");
            let mut value_hasher = ComponentHasher::new();
            value_hasher.put_bool(bold);
            entries.push(HashEntry::new(key_hasher, value_hasher));
        }

        // italic
        if let Some(italic) = self.italic {
            let mut key_hasher = ComponentHasher::new();
            key_hasher.put_string("italic");
            let mut value_hasher = ComponentHasher::new();
            value_hasher.put_bool(italic);
            entries.push(HashEntry::new(key_hasher, value_hasher));
        }

        // underlined
        if let Some(underlined) = self.underlined {
            let mut key_hasher = ComponentHasher::new();
            key_hasher.put_string("underlined");
            let mut value_hasher = ComponentHasher::new();
            value_hasher.put_bool(underlined);
            entries.push(HashEntry::new(key_hasher, value_hasher));
        }

        // strikethrough
        if let Some(strikethrough) = self.strikethrough {
            let mut key_hasher = ComponentHasher::new();
            key_hasher.put_string("strikethrough");
            let mut value_hasher = ComponentHasher::new();
            value_hasher.put_bool(strikethrough);
            entries.push(HashEntry::new(key_hasher, value_hasher));
        }

        // obfuscated
        if let Some(obfuscated) = self.obfuscated {
            let mut key_hasher = ComponentHasher::new();
            key_hasher.put_string("obfuscated");
            let mut value_hasher = ComponentHasher::new();
            value_hasher.put_bool(obfuscated);
            entries.push(HashEntry::new(key_hasher, value_hasher));
        }

        // insertion
        if let Some(insertion) = &self.insertion {
            let mut key_hasher = ComponentHasher::new();
            key_hasher.put_string("insertion");
            let mut value_hasher = ComponentHasher::new();
            value_hasher.put_string(insertion);
            entries.push(HashEntry::new(key_hasher, value_hasher));
        }

        // font (encoded as a string identifier)
        if let Some(font) = &self.font {
            let mut key_hasher = ComponentHasher::new();
            key_hasher.put_string("font");
            let mut value_hasher = ComponentHasher::new();
            value_hasher.put_string(font);
            entries.push(HashEntry::new(key_hasher, value_hasher));
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

    /// Parses a `Style` from an NBT compound.
    #[must_use]
    pub fn from_nbt_compound(compound: &NbtCompound) -> Self {
        use simdnbt::owned::NbtTag;

        let mut style = Self::new();

        if let Some(NbtTag::String(color_str)) = compound.get("color") {
            style.color = Color::parse(&color_str.to_string());
        }

        if let Some(NbtTag::Byte(b)) = compound.get("bold") {
            style.bold = Some(*b != 0);
        }

        if let Some(NbtTag::Byte(b)) = compound.get("italic") {
            style.italic = Some(*b != 0);
        }

        if let Some(NbtTag::Byte(b)) = compound.get("underlined") {
            style.underlined = Some(*b != 0);
        }

        if let Some(NbtTag::Byte(b)) = compound.get("strikethrough") {
            style.strikethrough = Some(*b != 0);
        }

        if let Some(NbtTag::Byte(b)) = compound.get("obfuscated") {
            style.obfuscated = Some(*b != 0);
        }

        if let Some(NbtTag::String(s)) = compound.get("insertion") {
            style.insertion = Some(s.to_string());
        }

        if let Some(NbtTag::String(s)) = compound.get("font") {
            style.font = Some(s.to_string());
        }

        style
    }
}
