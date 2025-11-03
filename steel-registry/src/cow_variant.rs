use std::collections::HashMap;
use steel_utils::ResourceLocation;

use crate::RegistryExt;

/// Represents a full cow variant definition from a data pack JSON file.
#[derive(Debug)]
pub struct CowVariant {
    pub key: ResourceLocation,
    pub asset_id: ResourceLocation,
    pub model: CowModelType,
    pub spawn_conditions: &'static [SpawnConditionEntry],
}

/// The model type for the cow, which can affect its shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CowModelType {
    #[default]
    Normal,
    Cold,
    Warm,
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

pub type CowVariantRef = &'static CowVariant;

pub struct CowVariantRegistry {
    cow_variants_by_id: Vec<CowVariantRef>,
    cow_variants_by_key: HashMap<ResourceLocation, usize>,
    allows_registering: bool,
}

impl CowVariantRegistry {
    pub fn new() -> Self {
        Self {
            cow_variants_by_id: Vec::new(),
            cow_variants_by_key: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, cow_variant: CowVariantRef) -> usize {
        if !self.allows_registering {
            panic!("Cannot register cow variants after the registry has been frozen");
        }

        let id = self.cow_variants_by_id.len();
        self.cow_variants_by_key.insert(cow_variant.key.clone(), id);
        self.cow_variants_by_id.push(cow_variant);
        id
    }

    pub fn by_id(&self, id: usize) -> Option<CowVariantRef> {
        self.cow_variants_by_id.get(id).copied()
    }

    pub fn get_id(&self, cow_variant: CowVariantRef) -> &usize {
        self.cow_variants_by_key
            .get(&cow_variant.key)
            .expect("Cow variant not found")
    }

    pub fn by_key(&self, key: &ResourceLocation) -> Option<CowVariantRef> {
        self.cow_variants_by_key
            .get(key)
            .and_then(|id| self.by_id(*id))
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, CowVariantRef)> + '_ {
        self.cow_variants_by_id
            .iter()
            .enumerate()
            .map(|(id, &variant)| (id, variant))
    }

    pub fn len(&self) -> usize {
        self.cow_variants_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cow_variants_by_id.is_empty()
    }
}

impl RegistryExt for CowVariantRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

impl Default for CowVariantRegistry {
    fn default() -> Self {
        Self::new()
    }
}
