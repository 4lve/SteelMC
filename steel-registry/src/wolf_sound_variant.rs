use std::collections::HashMap;
use steel_utils::ResourceLocation;

use crate::RegistryExt;

/// Represents a set of sounds for a wolf variant from a data pack JSON file.
#[derive(Debug)]
pub struct WolfSoundVariant {
    pub key: ResourceLocation,
    pub ambient_sound: ResourceLocation,
    pub death_sound: ResourceLocation,
    pub growl_sound: ResourceLocation,
    pub hurt_sound: ResourceLocation,
    pub pant_sound: ResourceLocation,
    pub whine_sound: ResourceLocation,
}

pub type WolfSoundVariantRef = &'static WolfSoundVariant;

pub struct WolfSoundVariantRegistry {
    wolf_sound_variants_by_id: Vec<WolfSoundVariantRef>,
    wolf_sound_variants_by_key: HashMap<ResourceLocation, usize>,
    allows_registering: bool,
}

impl WolfSoundVariantRegistry {
    pub fn new() -> Self {
        Self {
            wolf_sound_variants_by_id: Vec::new(),
            wolf_sound_variants_by_key: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, wolf_sound_variant: WolfSoundVariantRef) -> usize {
        if !self.allows_registering {
            panic!("Cannot register wolf sound variants after the registry has been frozen");
        }

        let id = self.wolf_sound_variants_by_id.len();
        self.wolf_sound_variants_by_key
            .insert(wolf_sound_variant.key.clone(), id);
        self.wolf_sound_variants_by_id.push(wolf_sound_variant);
        id
    }

    pub fn by_id(&self, id: usize) -> Option<WolfSoundVariantRef> {
        self.wolf_sound_variants_by_id.get(id).copied()
    }

    pub fn get_id(&self, wolf_sound_variant: WolfSoundVariantRef) -> &usize {
        self.wolf_sound_variants_by_key
            .get(&wolf_sound_variant.key)
            .expect("Wolf sound variant not found")
    }

    pub fn by_key(&self, key: &ResourceLocation) -> Option<WolfSoundVariantRef> {
        self.wolf_sound_variants_by_key
            .get(key)
            .and_then(|id| self.by_id(*id))
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, WolfSoundVariantRef)> + '_ {
        self.wolf_sound_variants_by_id
            .iter()
            .enumerate()
            .map(|(id, &variant)| (id, variant))
    }

    pub fn len(&self) -> usize {
        self.wolf_sound_variants_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.wolf_sound_variants_by_id.is_empty()
    }
}

impl RegistryExt for WolfSoundVariantRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

impl Default for WolfSoundVariantRegistry {
    fn default() -> Self {
        Self::new()
    }
}
