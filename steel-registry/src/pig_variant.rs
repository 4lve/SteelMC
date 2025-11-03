use std::collections::HashMap;
use steel_utils::ResourceLocation;

use crate::RegistryExt;

/// Represents a full pig variant definition from a data pack JSON file.
#[derive(Debug)]
pub struct PigVariant {
    pub key: ResourceLocation,
    pub asset_id: ResourceLocation,
    pub model: PigModelType,
    pub spawn_conditions: &'static [SpawnConditionEntry],
}

/// The model type for the pig, which can affect its shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PigModelType {
    #[default]
    Normal,
    Cold,
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

pub type PigVariantRef = &'static PigVariant;

pub struct PigVariantRegistry {
    pig_variants_by_id: Vec<PigVariantRef>,
    pig_variants_by_key: HashMap<ResourceLocation, usize>,
    allows_registering: bool,
}

impl PigVariantRegistry {
    pub fn new() -> Self {
        Self {
            pig_variants_by_id: Vec::new(),
            pig_variants_by_key: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, pig_variant: PigVariantRef) -> usize {
        if !self.allows_registering {
            panic!("Cannot register pig variants after the registry has been frozen");
        }

        let id = self.pig_variants_by_id.len();
        self.pig_variants_by_key.insert(pig_variant.key.clone(), id);
        self.pig_variants_by_id.push(pig_variant);
        id
    }

    pub fn by_id(&self, id: usize) -> Option<PigVariantRef> {
        self.pig_variants_by_id.get(id).copied()
    }

    pub fn get_id(&self, pig_variant: PigVariantRef) -> &usize {
        self.pig_variants_by_key
            .get(&pig_variant.key)
            .expect("Pig variant not found")
    }

    pub fn by_key(&self, key: &ResourceLocation) -> Option<PigVariantRef> {
        self.pig_variants_by_key
            .get(key)
            .and_then(|id| self.by_id(*id))
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, PigVariantRef)> + '_ {
        self.pig_variants_by_id
            .iter()
            .enumerate()
            .map(|(id, &variant)| (id, variant))
    }

    pub fn len(&self) -> usize {
        self.pig_variants_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pig_variants_by_id.is_empty()
    }
}

impl RegistryExt for PigVariantRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

impl Default for PigVariantRegistry {
    fn default() -> Self {
        Self::new()
    }
}
