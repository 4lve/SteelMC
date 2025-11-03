use std::collections::HashMap;
use steel_utils::ResourceLocation;
use steel_utils::text::TextComponent;

use crate::RegistryExt;

/// Represents a painting variant definition from a data pack JSON file.
#[derive(Debug)]
pub struct PaintingVariant {
    pub key: ResourceLocation,
    pub width: i32,
    pub height: i32,
    pub asset_id: ResourceLocation,
    pub title: Option<TextComponent>,
    pub author: Option<TextComponent>,
}

pub type PaintingVariantRef = &'static PaintingVariant;

pub struct PaintingVariantRegistry {
    painting_variants_by_id: Vec<PaintingVariantRef>,
    painting_variants_by_key: HashMap<ResourceLocation, usize>,
    allows_registering: bool,
}

impl PaintingVariantRegistry {
    pub fn new() -> Self {
        Self {
            painting_variants_by_id: Vec::new(),
            painting_variants_by_key: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, painting_variant: PaintingVariantRef) -> usize {
        if !self.allows_registering {
            panic!("Cannot register painting variants after the registry has been frozen");
        }

        let id = self.painting_variants_by_id.len();
        self.painting_variants_by_key
            .insert(painting_variant.key.clone(), id);
        self.painting_variants_by_id.push(painting_variant);
        id
    }

    pub fn by_id(&self, id: usize) -> Option<PaintingVariantRef> {
        self.painting_variants_by_id.get(id).copied()
    }

    pub fn get_id(&self, painting_variant: PaintingVariantRef) -> &usize {
        self.painting_variants_by_key
            .get(&painting_variant.key)
            .expect("Painting variant not found")
    }

    pub fn by_key(&self, key: &ResourceLocation) -> Option<PaintingVariantRef> {
        self.painting_variants_by_key
            .get(key)
            .and_then(|id| self.by_id(*id))
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, PaintingVariantRef)> + '_ {
        self.painting_variants_by_id
            .iter()
            .enumerate()
            .map(|(id, &variant)| (id, variant))
    }

    pub fn len(&self) -> usize {
        self.painting_variants_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.painting_variants_by_id.is_empty()
    }
}

impl RegistryExt for PaintingVariantRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

impl Default for PaintingVariantRegistry {
    fn default() -> Self {
        Self::new()
    }
}
