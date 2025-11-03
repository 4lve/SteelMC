use std::collections::HashMap;
use steel_utils::ResourceLocation;

use crate::RegistryExt;

/// Represents an armor trim material definition from the data packs.
#[derive(Debug)]
pub struct TrimMaterial {
    pub key: ResourceLocation,
    pub asset_name: String,
    pub description: StyledTextComponent,
    pub override_armor_assets: HashMap<ResourceLocation, String>,
}

/// Represents a translatable text component that can also include styling.
#[derive(Debug)]
pub struct StyledTextComponent {
    pub translate: String,
    pub color: Option<String>,
}

pub type TrimMaterialRef = &'static TrimMaterial;

pub struct TrimMaterialRegistry {
    trim_materials_by_id: Vec<TrimMaterialRef>,
    trim_materials_by_key: HashMap<ResourceLocation, usize>,
    allows_registering: bool,
}

impl TrimMaterialRegistry {
    pub fn new() -> Self {
        Self {
            trim_materials_by_id: Vec::new(),
            trim_materials_by_key: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, trim_material: TrimMaterialRef, key: ResourceLocation) -> usize {
        if !self.allows_registering {
            panic!("Cannot register trim materials after the registry has been frozen");
        }

        let id = self.trim_materials_by_id.len();
        self.trim_materials_by_key.insert(key, id);
        self.trim_materials_by_id.push(trim_material);
        id
    }

    pub fn by_id(&self, id: usize) -> Option<TrimMaterialRef> {
        self.trim_materials_by_id.get(id).copied()
    }

    pub fn get_id(&self, trim_material: TrimMaterialRef) -> &usize {
        self.trim_materials_by_key
            .get(&trim_material.key)
            .expect("Trim material not found")
    }

    pub fn by_key(&self, key: &ResourceLocation) -> Option<TrimMaterialRef> {
        self.trim_materials_by_key
            .get(key)
            .and_then(|id| self.by_id(*id))
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, TrimMaterialRef)> + '_ {
        self.trim_materials_by_id
            .iter()
            .enumerate()
            .map(|(id, &material)| (id, material))
    }

    pub fn len(&self) -> usize {
        self.trim_materials_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.trim_materials_by_id.is_empty()
    }
}

impl RegistryExt for TrimMaterialRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

impl Default for TrimMaterialRegistry {
    fn default() -> Self {
        Self::new()
    }
}
