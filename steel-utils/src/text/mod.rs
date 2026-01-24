//! This module contains everything related to text components.
use std::{
    borrow::Cow,
    fmt::{self, Display},
    io::{Cursor, Error as IoError, Result as IoResult},
};

use serde::{Deserialize, Serialize};
use simdnbt::{
    ToNbtTag,
    owned::{NbtCompound, NbtList, NbtTag, read_tag},
};

use crate::{
    hash::{ComponentHasher, HashComponent, HashEntry},
    serial::ReadFrom,
    text::{
        color::Color, interactivity::Interactivity, style::Style, translation::TranslatedMessage,
    },
};

/// A module for colors.
pub mod color;
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
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.style = self.style.color(color.into());
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

    /// Parses a `TextComponent` from an NBT tag.
    #[must_use]
    pub fn from_nbt_tag(tag: &NbtTag) -> Option<Self> {
        match tag {
            NbtTag::String(s) => Some(Self::new().text(s.to_string())),
            NbtTag::Compound(compound) => Self::from_nbt_compound(compound),
            _ => None,
        }
    }

    /// Parses a `TextComponent` from an NBT compound.
    #[must_use]
    pub fn from_nbt_compound(compound: &NbtCompound) -> Option<Self> {
        let mut component = Self::new();

        // Parse content
        if let Some(NbtTag::String(text)) = compound.get("text") {
            component.content = TextContent::Text {
                text: Cow::Owned(text.to_string()),
            };
        } else if let Some(NbtTag::String(key)) = compound.get("translate") {
            let fallback = compound.get("fallback").and_then(|t| match t {
                NbtTag::String(s) => Some(Cow::Owned(s.to_string())),
                _ => None,
            });
            let args = compound.get("with").and_then(|t| match t {
                NbtTag::List(NbtList::Compound(list)) => {
                    Some(list.iter().filter_map(Self::from_nbt_compound).collect())
                }
                _ => None,
            });
            component.content = TextContent::Translate(TranslatedMessage {
                key: Cow::Owned(key.to_string()),
                fallback,
                args,
            });
        } else if let Some(NbtTag::String(keybind)) = compound.get("keybind") {
            component.content = TextContent::Keybind {
                keybind: Cow::Owned(keybind.to_string()),
            };
        }

        // Parse style
        component.style = Style::from_nbt_compound(compound);

        // Parse extra
        if let Some(NbtTag::List(NbtList::Compound(list))) = compound.get("extra") {
            component.extra = list.iter().filter_map(Self::from_nbt_compound).collect();
        }

        Some(component)
    }

    /// Converts this text component into an NBT compound.
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
impl simdnbt::FromNbtTag for TextComponent {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag) -> Option<Self> {
        // Convert borrowed tag to owned for parsing
        let owned = tag.to_owned();
        Self::from_nbt_tag(&owned)
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

impl ReadFrom for TextComponent {
    fn read(data: &mut Cursor<&[u8]>) -> IoResult<Self> {
        use crate::codec::VarInt;

        // Minecraft's network format: VarInt length prefix, then NBT tag data
        let nbt_length = VarInt::read(data)?.0 as usize;

        if nbt_length == 0 {
            // Empty NBT means empty/default text component
            return Ok(Self::new());
        }

        // Read exactly one NBT tag using simdnbt
        let nbt_tag =
            read_tag(data).map_err(|e| IoError::other(format!("Failed to read NBT: {e:?}")))?;

        Self::from_nbt_tag(&nbt_tag)
            .ok_or_else(|| IoError::other("Failed to parse TextComponent from NBT"))
    }
}

impl HashComponent for TextComponent {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        // Minecraft's CODEC for Component uses an Either:
        // - If the component is plain text only (no siblings, no style), encode as just a string
        // - Otherwise, encode as a full map structure
        //
        // This matches ComponentSerialization.createCodec's tryCollapseToString logic
        if self.can_collapse_to_string() {
            // Simple text - hash as just a string
            if let TextContent::Text { text } = &self.content {
                hasher.put_string(text);
            }
        } else {
            // Complex component - hash as a map structure
            self.hash_as_map(hasher);
        }
    }
}

impl TextComponent {
    /// Check if this component can be collapsed to a plain string.
    /// This matches Minecraft's `tryCollapseToString` logic.
    fn can_collapse_to_string(&self) -> bool {
        matches!(&self.content, TextContent::Text { .. })
            && self.extra.is_empty()
            && self.style.is_empty()
            && self.interactivity.is_empty()
    }

    /// Hash this component as a map structure (for non-collapsible components).
    fn hash_as_map(&self, hasher: &mut ComponentHasher) {
        use crate::hash::sort_map_entries;

        // Collect all map entries with their key and value hashes for sorting
        let mut entries: Vec<HashEntry> = Vec::new();

        // Hash content
        match &self.content {
            TextContent::Text { text } => {
                let mut key_hasher = ComponentHasher::new();
                key_hasher.put_string("text");
                let mut value_hasher = ComponentHasher::new();
                value_hasher.put_string(text);
                entries.push(HashEntry::new(key_hasher, value_hasher));
            }
            TextContent::Translate(message) => {
                // "translate" field
                {
                    let mut key_hasher = ComponentHasher::new();
                    key_hasher.put_string("translate");
                    let mut value_hasher = ComponentHasher::new();
                    value_hasher.put_string(&message.key);
                    entries.push(HashEntry::new(key_hasher, value_hasher));
                }
                // "fallback" field (optional)
                if let Some(fallback) = &message.fallback {
                    let mut key_hasher = ComponentHasher::new();
                    key_hasher.put_string("fallback");
                    let mut value_hasher = ComponentHasher::new();
                    value_hasher.put_string(fallback);
                    entries.push(HashEntry::new(key_hasher, value_hasher));
                }
                // "with" field (optional args list)
                if let Some(args) = &message.args
                    && !args.is_empty()
                {
                    let mut key_hasher = ComponentHasher::new();
                    key_hasher.put_string("with");
                    let mut value_hasher = ComponentHasher::new();
                    value_hasher.start_list();
                    for arg in args {
                        let mut arg_hasher = ComponentHasher::new();
                        arg.hash_component(&mut arg_hasher);
                        value_hasher.put_raw_bytes(arg_hasher.current_data());
                    }
                    value_hasher.end_list();
                    entries.push(HashEntry::new(key_hasher, value_hasher));
                }
            }
            TextContent::Keybind { keybind } => {
                let mut key_hasher = ComponentHasher::new();
                key_hasher.put_string("keybind");
                let mut value_hasher = ComponentHasher::new();
                value_hasher.put_string(keybind);
                entries.push(HashEntry::new(key_hasher, value_hasher));
            }
        }

        // Hash style fields
        self.style.hash_fields(&mut entries);

        // Hash extra (siblings)
        if !self.extra.is_empty() {
            let mut key_hasher = ComponentHasher::new();
            key_hasher.put_string("extra");
            let mut value_hasher = ComponentHasher::new();
            value_hasher.start_list();
            for extra in &self.extra {
                let mut extra_hasher = ComponentHasher::new();
                extra.hash_component(&mut extra_hasher);
                value_hasher.put_raw_bytes(extra_hasher.current_data());
            }
            value_hasher.end_list();
            entries.push(HashEntry::new(key_hasher, value_hasher));
        }

        // Sort entries by key hash, then value hash (Minecraft's map ordering)
        sort_map_entries(&mut entries);

        // Write the sorted map
        hasher.start_map();
        for entry in entries {
            hasher.put_raw_bytes(&entry.key_bytes);
            hasher.put_raw_bytes(&entry.value_bytes);
        }
        hasher.end_map();
    }
}
