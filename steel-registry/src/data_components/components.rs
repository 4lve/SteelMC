use std::{any::Any, collections::HashMap, fmt::Debug, marker::PhantomData};

use steel_utils::Identifier;

use crate::{
    RegistryExt,
    data_components::vanilla_components::{
        ATTRIBUTE_MODIFIERS, BREAK_SOUND, ENCHANTMENTS, LORE, MAX_STACK_SIZE, RARITY, REPAIR_COST,
        TOOLTIP_DISPLAY,
    },
};

pub trait ComponentValue: Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static + Send + Sync + Debug> ComponentValue for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

//TODO: Implement codecs, also one for persistent storage and one for network.
pub struct DataComponentType<T> {
    pub key: Identifier,
    _phantom: PhantomData<T>,
}

impl<T> DataComponentType<T> {
    #[must_use]
    pub const fn new(key: Identifier) -> Self {
        Self {
            key,
            _phantom: PhantomData,
        }
    }
}

pub struct DataComponentRegistry {
    components_by_key: HashMap<Identifier, usize>,
    allows_registering: bool,
}

impl Default for DataComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DataComponentRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            components_by_key: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn register<T: 'static>(&mut self, component: DataComponentType<T>) {
        assert!(
            self.allows_registering,
            "Cannot register data components after the registry has been frozen"
        );

        let id = self.components_by_key.len();
        self.components_by_key.insert(component.key.clone(), id);
    }

    #[must_use]
    pub fn get_id<T: 'static>(&self, component: DataComponentType<T>) -> Option<usize> {
        self.components_by_key.get(&component.key).copied()
    }
}

impl RegistryExt for DataComponentRegistry {
    // Prevents the registry from registering new blocks.
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

#[derive(Debug)]
pub struct DataComponentMap {
    map: Vec<(Identifier, Box<dyn ComponentValue>)>,
}

impl Default for DataComponentMap {
    fn default() -> Self {
        Self::new()
    }
}

impl DataComponentMap {
    #[must_use]
    pub const fn new() -> Self {
        Self { map: Vec::new() }
    }

    #[must_use]
    pub fn common_item_components() -> Self {
        //TODO: Some components still have todo values, we should implement them

        Self {
            map: vec![
                (MAX_STACK_SIZE.key.clone(), Box::new(64)),
                (LORE.key.clone(), Box::new(())),
                (ENCHANTMENTS.key.clone(), Box::new(())),
                (REPAIR_COST.key.clone(), Box::new(0)),
                (ATTRIBUTE_MODIFIERS.key.clone(), Box::new(())),
                (RARITY.key.clone(), Box::new(())),
                (BREAK_SOUND.key.clone(), Box::new(())),
                (TOOLTIP_DISPLAY.key.clone(), Box::new(())),
            ],
        }
    }

    #[must_use]
    pub fn builder_set<T: 'static + ComponentValue>(
        mut self,
        component: DataComponentType<T>,
        data: Option<T>,
    ) -> Self {
        self.set(component, data);
        self
    }

    pub fn set<T: 'static + ComponentValue>(
        &mut self,
        component: DataComponentType<T>,
        data: Option<T>,
    ) {
        if let Some(data) = data {
            self.map.push((component.key.clone(), Box::new(data)));
        } else if let Some(index) = self
            .map
            .iter()
            .position(|(res_loc, _)| *res_loc == component.key)
        {
            self.map.swap_remove(index);
        }
    }

    #[must_use]
    pub fn get<T: 'static>(&self, component: DataComponentType<T>) -> Option<&T> {
        let index = self
            .map
            .iter()
            .position(|(res_loc, _)| *res_loc == component.key)?;
        self.map[index].as_any().downcast_ref::<T>()
    }

    #[must_use]
    pub fn has<T: 'static>(&self, component: DataComponentType<T>) -> bool {
        self.map
            .iter()
            .any(|(res_loc, _)| *res_loc == component.key)
    }
}
