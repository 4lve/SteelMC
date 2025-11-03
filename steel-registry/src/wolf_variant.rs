use std::collections::HashMap;
use steel_utils::ResourceLocation;

use crate::RegistryExt;

/// Represents a full wolf variant definition from a data pack JSON file.
#[derive(Debug)]
pub struct WolfVariant {
    pub key: ResourceLocation,
    pub assets: WolfAssetInfo,
    pub spawn_conditions: &'static [SpawnConditionEntry],
}

/// Contains the texture resource locations for a wolf variant.
#[derive(Debug)]
pub struct WolfAssetInfo {
    pub wild: ResourceLocation,
    pub tame: ResourceLocation,
    pub angry: ResourceLocation,
}

/// A single entry in the list of spawn conditions.
#[derive(Debug)]
pub struct SpawnConditionEntry {
    pub priority: i32,
    pub condition: Option<BiomeCondition>,
}

/// Defines a condition based on a biome or list of biomes.
#[derive(Debug)]
pub struct BiomeCondition {
    pub condition_type: &'static str,
    pub biomes: &'static str,
}

pub type WolfVariantRef = &'static WolfVariant;

pub struct WolfVariantRegistry {
    wolf_variants_by_id: Vec<WolfVariantRef>,
    wolf_variants_by_key: HashMap<ResourceLocation, usize>,
    allows_registering: bool,
}

impl WolfVariantRegistry {
    pub fn new() -> Self {
        Self {
            wolf_variants_by_id: Vec::new(),
            wolf_variants_by_key: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, wolf_variant: WolfVariantRef) -> usize {
        if !self.allows_registering {
            panic!("Cannot register wolf variants after the registry has been frozen");
        }

        let id = self.wolf_variants_by_id.len();
        self.wolf_variants_by_key
            .insert(wolf_variant.key.clone(), id);
        self.wolf_variants_by_id.push(wolf_variant);
        id
    }

    pub fn by_id(&self, id: usize) -> Option<WolfVariantRef> {
        self.wolf_variants_by_id.get(id).copied()
    }

    pub fn get_id(&self, wolf_variant: WolfVariantRef) -> &usize {
        self.wolf_variants_by_key
            .get(&wolf_variant.key)
            .expect("Wolf variant not found")
    }

    pub fn by_key(&self, key: &ResourceLocation) -> Option<WolfVariantRef> {
        self.wolf_variants_by_key
            .get(key)
            .and_then(|id| self.by_id(*id))
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, WolfVariantRef)> + '_ {
        self.wolf_variants_by_id
            .iter()
            .enumerate()
            .map(|(id, &variant)| (id, variant))
    }

    pub fn len(&self) -> usize {
        self.wolf_variants_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.wolf_variants_by_id.is_empty()
    }
}

impl RegistryExt for WolfVariantRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

impl Default for WolfVariantRegistry {
    fn default() -> Self {
        Self::new()
    }
}
