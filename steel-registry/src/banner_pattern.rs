use std::collections::HashMap;
use steel_utils::ResourceLocation;

use crate::RegistryExt;

/// Represents a banner pattern definition from a data pack JSON file.
#[derive(Debug)]
pub struct BannerPattern {
    pub key: ResourceLocation,
    pub asset_id: ResourceLocation,
    pub translation_key: &'static str,
}

pub type BannerPatternRef = &'static BannerPattern;

pub struct BannerPatternRegistry {
    banner_patterns_by_id: Vec<BannerPatternRef>,
    banner_patterns_by_key: HashMap<ResourceLocation, usize>,
    allows_registering: bool,
}

impl BannerPatternRegistry {
    pub fn new() -> Self {
        Self {
            banner_patterns_by_id: Vec::new(),
            banner_patterns_by_key: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, banner_pattern: BannerPatternRef) -> usize {
        if !self.allows_registering {
            panic!("Cannot register banner patterns after the registry has been frozen");
        }

        let id = self.banner_patterns_by_id.len();
        self.banner_patterns_by_key
            .insert(banner_pattern.key.clone(), id);
        self.banner_patterns_by_id.push(banner_pattern);
        id
    }

    pub fn by_id(&self, id: usize) -> Option<BannerPatternRef> {
        self.banner_patterns_by_id.get(id).copied()
    }

    pub fn get_id(&self, banner_pattern: BannerPatternRef) -> &usize {
        self.banner_patterns_by_key
            .get(&banner_pattern.key)
            .expect("Banner pattern not found")
    }

    pub fn by_key(&self, key: &ResourceLocation) -> Option<BannerPatternRef> {
        self.banner_patterns_by_key
            .get(key)
            .and_then(|id| self.by_id(*id))
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, BannerPatternRef)> + '_ {
        self.banner_patterns_by_id
            .iter()
            .enumerate()
            .map(|(id, &pattern)| (id, pattern))
    }

    pub fn len(&self) -> usize {
        self.banner_patterns_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.banner_patterns_by_id.is_empty()
    }
}

impl Default for BannerPatternRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryExt for BannerPatternRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}
