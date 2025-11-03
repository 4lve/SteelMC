use std::collections::HashMap;

use steel_utils::ResourceLocation;

use crate::{RegistryExt, blocks::BlockRef, data_components::DataComponentMap};

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
    tags: HashMap<ResourceLocation, Vec<ItemRef>>,
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
            tags: HashMap::new(),
            allows_registering: true,
        }
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

    pub fn iter(&self) -> impl Iterator<Item = (usize, ItemRef)> + '_ {
        self.items_by_id
            .iter()
            .enumerate()
            .map(|(id, &item)| (id, item))
    }

    pub fn len(&self) -> usize {
        self.items_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items_by_id.is_empty()
    }

    // Tag-related methods

    /// Registers a tag with a list of item keys.
    /// Item keys that don't exist in the registry are silently skipped.
    pub fn register_tag(&mut self, tag: ResourceLocation, item_keys: &[&'static str]) {
        if !self.allows_registering {
            panic!("Cannot register tags after registry has been frozen");
        }

        let items: Vec<ItemRef> = item_keys
            .iter()
            .filter_map(|key| self.by_key(&ResourceLocation::vanilla_static(key)))
            .collect();

        self.tags.insert(tag, items);
    }

    /// Checks if an item is in a given tag.
    pub fn is_in_tag(&self, item: ItemRef, tag: &ResourceLocation) -> bool {
        self.tags
            .get(tag)
            .map(|items| {
                items
                    .iter()
                    .any(|&i| std::ptr::eq(i as *const _, item as *const _))
            })
            .unwrap_or(false)
    }

    /// Gets all items in a tag.
    pub fn get_tag(&self, tag: &ResourceLocation) -> Option<&[ItemRef]> {
        self.tags.get(tag).map(|v| v.as_slice())
    }

    /// Iterates over all items in a tag.
    pub fn iter_tag(&self, tag: &ResourceLocation) -> impl Iterator<Item = ItemRef> + '_ {
        self.tags
            .get(tag)
            .map(|v| v.iter().copied())
            .into_iter()
            .flatten()
    }

    /// Gets all tag keys.
    pub fn tag_keys(&self) -> impl Iterator<Item = &ResourceLocation> + '_ {
        self.tags.keys()
    }
}

impl RegistryExt for ItemRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}
