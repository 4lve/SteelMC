//! This module contains everything related to text components.
use serde::{Deserialize, Serialize};
use simdnbt::{
    ToNbtTag,
    owned::{NbtCompound, NbtList, NbtTag},
};
use std::{
    borrow::Cow,
    fmt::{self, Display},
    io::Write,
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

    /// Encodes the text component to NBT bytes for network transmission.
    /// Uses network NBT format: `TAG_Compound` byte, no name, then content.
    ///
    /// # Panics
    ///
    /// Panics if the text component fails to serialize to an NBT compound or if
    /// writing the NBT compound to bytes fails.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let nbt_tag = simdnbt::ToNbtTag::to_nbt_tag(self.clone());
        log::debug!("TextComponent NBT tag: {nbt_tag:?}");
        match nbt_tag {
            NbtTag::Compound(compound) => {
                let mut buffer = Vec::new();
                // Network NBT format per NbtIo.writeAnyTag: TAG byte + content
                buffer.push(0x0A); // TAG_Compound
                Self::write_nbt_compound(&mut buffer, &compound)
                    .expect("Failed to write NBT compound");
                log::debug!(
                    "Encoded NBT bytes (len={}): {:02X?}",
                    buffer.len(),
                    &buffer[..buffer.len().min(50)]
                );
                buffer
            }
            _ => panic!("TextComponent must serialize to NBT compound"),
        }
    }

    /// Helper to write NBT compound content
    fn write_nbt_compound(writer: &mut Vec<u8>, compound: &NbtCompound) -> std::io::Result<()> {
        for (key, value) in compound.iter() {
            // Write tag type
            writer.write_all(&[Self::get_nbt_tag_id(value)])?;
            // Write key as modified UTF-8 string
            let key_bytes = key.as_bytes();
            writer.write_all(&(key_bytes.len() as u16).to_be_bytes())?;
            writer.write_all(key_bytes)?;
            // Write value payload
            Self::write_nbt_tag_payload(writer, value)?;
        }
        // Write TAG_End
        writer.write_all(&[0x00])?;
        Ok(())
    }

    fn get_nbt_tag_id(tag: &NbtTag) -> u8 {
        match tag {
            NbtTag::Byte(_) => 0x01,
            NbtTag::Short(_) => 0x02,
            NbtTag::Int(_) => 0x03,
            NbtTag::Long(_) => 0x04,
            NbtTag::Float(_) => 0x05,
            NbtTag::Double(_) => 0x06,
            NbtTag::ByteArray(_) => 0x07,
            NbtTag::String(_) => 0x08,
            NbtTag::List(_) => 0x09,
            NbtTag::Compound(_) => 0x0A,
            NbtTag::IntArray(_) => 0x0B,
            NbtTag::LongArray(_) => 0x0C,
        }
    }

    fn write_nbt_tag_payload(writer: &mut Vec<u8>, tag: &NbtTag) -> std::io::Result<()> {
        match tag {
            NbtTag::Byte(v) => writer.write_all(&[*v as u8])?,
            NbtTag::Short(v) => writer.write_all(&v.to_be_bytes())?,
            NbtTag::Int(v) => writer.write_all(&v.to_be_bytes())?,
            NbtTag::Long(v) => writer.write_all(&v.to_be_bytes())?,
            NbtTag::Float(v) => writer.write_all(&v.to_be_bytes())?,
            NbtTag::Double(v) => writer.write_all(&v.to_be_bytes())?,
            NbtTag::ByteArray(v) => {
                writer.write_all(&(v.len() as i32).to_be_bytes())?;
                writer.write_all(v)?;
            }
            NbtTag::String(v) => {
                let bytes = v.as_bytes();
                writer.write_all(&(bytes.len() as u16).to_be_bytes())?;
                writer.write_all(bytes)?;
            }
            NbtTag::List(list) => Self::write_nbt_list(writer, list)?,
            NbtTag::Compound(compound) => Self::write_nbt_compound(writer, compound)?,
            NbtTag::IntArray(v) => {
                writer.write_all(&(v.len() as i32).to_be_bytes())?;
                for int in v {
                    writer.write_all(&int.to_be_bytes())?;
                }
            }
            NbtTag::LongArray(v) => {
                writer.write_all(&(v.len() as i32).to_be_bytes())?;
                for long in v {
                    writer.write_all(&long.to_be_bytes())?;
                }
            }
        }
        Ok(())
    }

    fn write_nbt_list(writer: &mut Vec<u8>, list: &NbtList) -> std::io::Result<()> {
        match list {
            NbtList::Empty => {
                writer.write_all(&[0x00])?; // TAG_End
                writer.write_all(&[0x00, 0x00, 0x00, 0x00])?; // Length 0
            }
            NbtList::Byte(v) => {
                writer.write_all(&[0x01])?;
                writer.write_all(&(v.len() as i32).to_be_bytes())?;
                for b in v {
                    writer.write_all(&[*b as u8])?;
                }
            }
            NbtList::Short(v) => {
                writer.write_all(&[0x02])?;
                writer.write_all(&(v.len() as i32).to_be_bytes())?;
                for s in v {
                    writer.write_all(&s.to_be_bytes())?;
                }
            }
            NbtList::Int(v) => {
                writer.write_all(&[0x03])?;
                writer.write_all(&(v.len() as i32).to_be_bytes())?;
                for i in v {
                    writer.write_all(&i.to_be_bytes())?;
                }
            }
            NbtList::Long(v) => Self::write_nbt_list_long(writer, v)?,
            NbtList::Float(v) => Self::write_nbt_list_float(writer, v)?,
            NbtList::Double(v) => Self::write_nbt_list_double(writer, v)?,
            NbtList::ByteArray(v) => Self::write_nbt_list_byte_array(writer, v)?,
            NbtList::String(v) => Self::write_nbt_list_string(writer, v)?,
            NbtList::List(v) => Self::write_nbt_list_list(writer, v)?,
            NbtList::Compound(v) => Self::write_nbt_list_compound(writer, v)?,
            NbtList::IntArray(v) => Self::write_nbt_list_int_array(writer, v)?,
            NbtList::LongArray(v) => Self::write_nbt_list_long_array(writer, v)?,
        }
        Ok(())
    }

    fn write_nbt_list_long(writer: &mut Vec<u8>, v: &[i64]) -> std::io::Result<()> {
        writer.write_all(&[0x04])?;
        writer.write_all(&(v.len() as i32).to_be_bytes())?;
        for l in v {
            writer.write_all(&l.to_be_bytes())?;
        }
        Ok(())
    }

    fn write_nbt_list_float(writer: &mut Vec<u8>, v: &[f32]) -> std::io::Result<()> {
        writer.write_all(&[0x05])?;
        writer.write_all(&(v.len() as i32).to_be_bytes())?;
        for f in v {
            writer.write_all(&f.to_be_bytes())?;
        }
        Ok(())
    }

    fn write_nbt_list_double(writer: &mut Vec<u8>, v: &[f64]) -> std::io::Result<()> {
        writer.write_all(&[0x06])?;
        writer.write_all(&(v.len() as i32).to_be_bytes())?;
        for d in v {
            writer.write_all(&d.to_be_bytes())?;
        }
        Ok(())
    }

    fn write_nbt_list_byte_array(writer: &mut Vec<u8>, v: &[Vec<u8>]) -> std::io::Result<()> {
        writer.write_all(&[0x07])?;
        writer.write_all(&(v.len() as i32).to_be_bytes())?;
        for arr in v {
            writer.write_all(&(arr.len() as i32).to_be_bytes())?;
            writer.write_all(arr)?;
        }
        Ok(())
    }

    fn write_nbt_list_string(
        writer: &mut Vec<u8>,
        v: &[simdnbt::Mutf8String],
    ) -> std::io::Result<()> {
        writer.write_all(&[0x08])?;
        writer.write_all(&(v.len() as i32).to_be_bytes())?;
        for s in v {
            let bytes = s.as_bytes();
            writer.write_all(&(bytes.len() as u16).to_be_bytes())?;
            writer.write_all(bytes)?;
        }
        Ok(())
    }

    fn write_nbt_list_list(writer: &mut Vec<u8>, v: &[NbtList]) -> std::io::Result<()> {
        writer.write_all(&[0x09])?;
        writer.write_all(&(v.len() as i32).to_be_bytes())?;
        for l in v {
            Self::write_nbt_list(writer, l)?;
        }
        Ok(())
    }

    fn write_nbt_list_compound(writer: &mut Vec<u8>, v: &[NbtCompound]) -> std::io::Result<()> {
        writer.write_all(&[0x0A])?;
        writer.write_all(&(v.len() as i32).to_be_bytes())?;
        for c in v {
            Self::write_nbt_compound(writer, c)?;
        }
        Ok(())
    }

    fn write_nbt_list_int_array(writer: &mut Vec<u8>, v: &[Vec<i32>]) -> std::io::Result<()> {
        writer.write_all(&[0x0B])?;
        writer.write_all(&(v.len() as i32).to_be_bytes())?;
        for arr in v {
            writer.write_all(&(arr.len() as i32).to_be_bytes())?;
            for i in arr {
                writer.write_all(&i.to_be_bytes())?;
            }
        }
        Ok(())
    }

    fn write_nbt_list_long_array(writer: &mut Vec<u8>, v: &[Vec<i64>]) -> std::io::Result<()> {
        writer.write_all(&[0x0C])?;
        writer.write_all(&(v.len() as i32).to_be_bytes())?;
        for arr in v {
            writer.write_all(&(arr.len() as i32).to_be_bytes())?;
            for l in arr {
                writer.write_all(&l.to_be_bytes())?;
            }
        }
        Ok(())
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
