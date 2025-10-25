use std::{collections::HashMap, marker::PhantomData};

use steel_utils::ResourceLocation;

use crate::data_components::vanilla_components::{
    ATTRIBUTE_MODIFIERS, BREAK_SOUND, ENCHANTMENTS, LORE, MAX_STACK_SIZE, RARITY, REPAIR_COST,
    TOOLTIP_DISPLAY,
};

pub trait ComponentValue: std::fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<T: 'static + Send + Sync + std::fmt::Debug> ComponentValue for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

//TODO: Implement codecs, also one for persistent storage and one for network.
pub struct DataComponentType<T> {
    pub key: ResourceLocation,
    _phantom: PhantomData<T>,
}

impl<T> DataComponentType<T> {
    pub const fn new(key: ResourceLocation) -> Self {
        Self {
            key,
            _phantom: PhantomData,
        }
    }
}

pub struct DataComponentRegistry {
    components_by_key: HashMap<ResourceLocation, usize>,
    allows_registering: bool,
}

impl Default for DataComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DataComponentRegistry {
    pub fn new() -> Self {
        Self {
            components_by_key: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn freeze(&mut self) {
        self.allows_registering = false;
    }

    pub fn register<T: 'static>(&mut self, component: &'static DataComponentType<T>) {
        if !self.allows_registering {
            panic!("Cannot register data components after the registry has been frozen");
        }

        let id = self.components_by_key.len();
        self.components_by_key.insert(component.key.clone(), id);
    }

    pub fn get_id<T: 'static>(&self, component: &DataComponentType<T>) -> Option<usize> {
        self.components_by_key.get(&component.key).copied()
    }
}

#[derive(Debug)]
pub struct DataComponentMap {
    map: HashMap<ResourceLocation, Box<dyn ComponentValue>>,
}

impl Default for DataComponentMap {
    fn default() -> Self {
        Self::new()
    }
}

impl DataComponentMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn common_item_components() -> Self {
        //TODO: Some components stil have todo values, we should implement them
        Self::new()
            .builder_set(MAX_STACK_SIZE, Some(64))
            .builder_set(LORE, Some(()))
            .builder_set(ENCHANTMENTS, Some(()))
            .builder_set(REPAIR_COST, Some(0))
            .builder_set(ATTRIBUTE_MODIFIERS, Some(()))
            .builder_set(RARITY, Some(()))
            .builder_set(BREAK_SOUND, Some(()))
            .builder_set(TOOLTIP_DISPLAY, Some(()))
    }

    pub fn builder_set<T: 'static + ComponentValue>(
        mut self,
        component: &DataComponentType<T>,
        data: Option<T>,
    ) -> Self {
        self.set(component, data);
        self
    }

    pub fn set<T: 'static + ComponentValue>(
        &mut self,
        component: &DataComponentType<T>,
        data: Option<T>,
    ) {
        if let Some(data) = data {
            self.map.insert(component.key.clone(), Box::new(data));
        } else {
            self.map.remove(&component.key);
        }
    }

    pub fn get<T: 'static>(&self, component: &DataComponentType<T>) -> Option<&T> {
        self.map
            .get(&component.key)
            .and_then(|data| data.as_any().downcast_ref::<T>())
    }

    pub fn has<T: 'static>(&self, component: &DataComponentType<T>) -> bool {
        self.map.contains_key(&component.key)
    }
}
