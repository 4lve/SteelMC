use std::collections::HashMap;
use steel_utils::ResourceLocation;

use crate::RegistryExt;

/// Represents a full cat variant definition from a data pack JSON file.
#[derive(Debug)]
pub struct CatVariant {
    pub key: ResourceLocation,
    pub asset_id: ResourceLocation,
    pub spawn_conditions: &'static [SpawnConditionEntry],
}

/// A single entry in the list of spawn conditions.
#[derive(Debug)]
pub struct SpawnConditionEntry {
    pub priority: i32,
    pub condition: Option<SpawnCondition>,
}

/// Defines various spawn conditions for cat variants.
#[derive(Debug)]
pub enum SpawnCondition {
    Structure { structures: &'static str },
    MoonBrightness { min: Option<f32>, max: Option<f32> },
    Biome { biomes: &'static str },
}

pub type CatVariantRef = &'static CatVariant;

pub struct CatVariantRegistry {
    cat_variants_by_id: Vec<CatVariantRef>,
    cat_variants_by_key: HashMap<ResourceLocation, usize>,
    allows_registering: bool,
}

impl CatVariantRegistry {
    pub fn new() -> Self {
        Self {
            cat_variants_by_id: Vec::new(),
            cat_variants_by_key: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, cat_variant: CatVariantRef) -> usize {
        if !self.allows_registering {
            panic!("Cannot register cat variants after the registry has been frozen");
        }

        let id = self.cat_variants_by_id.len();
        self.cat_variants_by_key.insert(cat_variant.key.clone(), id);
        self.cat_variants_by_id.push(cat_variant);
        id
    }

    pub fn by_id(&self, id: usize) -> Option<CatVariantRef> {
        self.cat_variants_by_id.get(id).copied()
    }

    pub fn get_id(&self, cat_variant: CatVariantRef) -> &usize {
        self.cat_variants_by_key
            .get(&cat_variant.key)
            .expect("Cat variant not found")
    }

    pub fn by_key(&self, key: &ResourceLocation) -> Option<CatVariantRef> {
        self.cat_variants_by_key
            .get(key)
            .and_then(|id| self.by_id(*id))
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, CatVariantRef)> + '_ {
        self.cat_variants_by_id
            .iter()
            .enumerate()
            .map(|(id, &variant)| (id, variant))
    }

    pub fn len(&self) -> usize {
        self.cat_variants_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cat_variants_by_id.is_empty()
    }
}

impl RegistryExt for CatVariantRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

impl Default for CatVariantRegistry {
    fn default() -> Self {
        Self::new()
    }
}
