use std::collections::HashMap;
use steel_utils::ResourceLocation;
use steel_utils::text::TextComponent;

use crate::RegistryExt;

/// Represents an armor trim pattern definition from the data packs.
#[derive(Debug)]
pub struct TrimPattern {
    pub key: ResourceLocation,
    pub asset_id: ResourceLocation,
    pub description: TextComponent,
    pub decal: bool,
}

pub type TrimPatternRef = &'static TrimPattern;

pub struct TrimPatternRegistry {
    trim_patterns_by_id: Vec<TrimPatternRef>,
    trim_patterns_by_key: HashMap<ResourceLocation, usize>,
    allows_registering: bool,
}

impl TrimPatternRegistry {
    pub fn new() -> Self {
        Self {
            trim_patterns_by_id: Vec::new(),
            trim_patterns_by_key: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, trim_pattern: TrimPatternRef) -> usize {
        if !self.allows_registering {
            panic!("Cannot register trim patterns after the registry has been frozen");
        }

        let id = self.trim_patterns_by_id.len();
        self.trim_patterns_by_key
            .insert(trim_pattern.key.clone(), id);
        self.trim_patterns_by_id.push(trim_pattern);
        id
    }

    pub fn by_id(&self, id: usize) -> Option<TrimPatternRef> {
        self.trim_patterns_by_id.get(id).copied()
    }

    pub fn get_id(&self, trim_pattern: TrimPatternRef) -> &usize {
        self.trim_patterns_by_key
            .get(&trim_pattern.key)
            .expect("Trim pattern not found")
    }

    pub fn by_key(&self, key: &ResourceLocation) -> Option<TrimPatternRef> {
        self.trim_patterns_by_key
            .get(key)
            .and_then(|id| self.by_id(*id))
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, TrimPatternRef)> + '_ {
        self.trim_patterns_by_id
            .iter()
            .enumerate()
            .map(|(id, &pattern)| (id, pattern))
    }

    pub fn len(&self) -> usize {
        self.trim_patterns_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.trim_patterns_by_id.is_empty()
    }
}

impl RegistryExt for TrimPatternRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

impl Default for TrimPatternRegistry {
    fn default() -> Self {
        Self::new()
    }
}
