//! This module contains everything related to text components.
use crate::translations_registry::TRANSLATIONS;
use text_components::{
    TextComponent,
    content::{Content, Resolvable},
    custom::CustomData,
    resolving::TextResolutor,
};

/// A [`TextResolutor`] for the console
pub struct DisplayResolutor;
impl TextResolutor for DisplayResolutor {
    fn resolve_content(&self, resolvable: &Resolvable) -> TextComponent {
        TextComponent {
            content: Content::Resolvable(resolvable.clone()),
            ..Default::default()
        }
    }

    fn resolve_custom(&self, _data: &CustomData) -> Option<TextComponent> {
        None
    }

    fn translate(&self, key: &str) -> Option<String> {
        TRANSLATIONS.get(key).map(ToString::to_string)
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
