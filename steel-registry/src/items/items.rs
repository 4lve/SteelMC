use std::collections::HashMap;

use steel_utils::ResourceLocation;

use crate::{blocks::blocks::BlockRef, data_components::DataComponentMap};

#[derive(Debug)]
pub struct Item {
    pub key: ResourceLocation,
    pub components: DataComponentMap,
}

impl Item {
    pub fn from_block(block: BlockRef) -> Self {
        Self {
            key: block.key.clone(),
            components: DataComponentMap::common_item_components(),
        }
    }

    pub fn from_block_custom_name(_block: BlockRef, name: &'static str) -> Self {
        Self {
            key: ResourceLocation::vanilla_static(name),
            components: DataComponentMap::common_item_components(),
        }
    }
}

pub type ItemRef = &'static Item;

pub struct ItemRegistry {
    items_by_id: Vec<ItemRef>,
    items_by_key: HashMap<ResourceLocation, usize>,
    allows_registering: bool,
}

impl Default for ItemRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemRegistry {
    pub fn new() -> Self {
        Self {
            items_by_id: Vec::new(),
            items_by_key: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn freeze(&mut self) {
        self.allows_registering = false;
    }

    pub fn register(&mut self, item: ItemRef) -> usize {
        if !self.allows_registering {
            panic!("Cannot register items after the registry has been frozen");
        }

        let id = self.items_by_id.len();
        self.items_by_key.insert(item.key.clone(), id);
        self.items_by_id.push(item);

        id
    }

    pub fn by_id(&self, id: usize) -> Option<ItemRef> {
        self.items_by_id.get(id).copied()
    }

    pub fn get_id(&self, item: ItemRef) -> &usize {
        self.items_by_key.get(&item.key).expect("Item not found")
    }

    pub fn by_key(&self, key: &ResourceLocation) -> Option<ItemRef> {
        self.items_by_key.get(key).and_then(|id| self.by_id(*id))
    }
}
