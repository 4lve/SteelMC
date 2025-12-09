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
    fn clone_boxed(&self) -> Box<dyn ComponentValue>;
    fn eq_value(&self, other: &dyn ComponentValue) -> bool;
}

impl<T: 'static + Send + Sync + Debug + Clone + PartialEq> ComponentValue for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn ComponentValue> {
        Box::new(self.clone())
    }

    fn eq_value(&self, other: &dyn ComponentValue) -> bool {
        other
            .as_any()
            .downcast_ref::<T>()
            .is_some_and(|o| self == o)
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

    #[must_use]
    pub fn get_id_by_key(&self, key: &Identifier) -> Option<usize> {
        self.components_by_key.get(key).copied()
    }
}

impl RegistryExt for DataComponentRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

#[derive(Debug)]
pub struct DataComponentMap {
    map: HashMap<Identifier, Box<dyn ComponentValue>>,
}

impl Default for DataComponentMap {
    fn default() -> Self {
        Self::new()
    }
}

impl DataComponentMap {
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    #[must_use]
    pub fn common_item_components() -> Self {
        let mut map = HashMap::new();
        map.insert(
            MAX_STACK_SIZE.key.clone(),
            Box::new(64i32) as Box<dyn ComponentValue>,
        );
        map.insert(LORE.key.clone(), Box::new(()) as Box<dyn ComponentValue>);
        map.insert(
            ENCHANTMENTS.key.clone(),
            Box::new(()) as Box<dyn ComponentValue>,
        );
        map.insert(
            REPAIR_COST.key.clone(),
            Box::new(0i32) as Box<dyn ComponentValue>,
        );
        map.insert(
            ATTRIBUTE_MODIFIERS.key.clone(),
            Box::new(()) as Box<dyn ComponentValue>,
        );
        map.insert(RARITY.key.clone(), Box::new(()) as Box<dyn ComponentValue>);
        map.insert(
            BREAK_SOUND.key.clone(),
            Box::new(()) as Box<dyn ComponentValue>,
        );
        map.insert(
            TOOLTIP_DISPLAY.key.clone(),
            Box::new(()) as Box<dyn ComponentValue>,
        );
        Self { map }
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
            self.map.insert(component.key.clone(), Box::new(data));
        } else {
            self.map.remove(&component.key);
        }
    }

    #[must_use]
    pub fn get<T: 'static>(&self, component: DataComponentType<T>) -> Option<&T> {
        let value = self.map.get(&component.key)?;
        value.as_ref().as_any().downcast_ref::<T>()
    }

    #[must_use]
    pub fn has<T: 'static>(&self, component: DataComponentType<T>) -> bool {
        self.map.contains_key(&component.key)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &Identifier> {
        self.map.keys()
    }

    #[must_use]
    pub fn get_raw(&self, key: &Identifier) -> Option<&dyn ComponentValue> {
        self.map.get(key).map(|v| v.as_ref())
    }
}

#[derive(Debug)]
pub enum ComponentPatchEntry {
    Set(Box<dyn ComponentValue>),
    Removed,
}

impl PartialEq for ComponentPatchEntry {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Removed, Self::Removed) => true,
            (Self::Set(a), Self::Set(b)) => a.eq_value(b.as_ref()),
            _ => false,
        }
    }
}

/// A patch representing modifications to a `DataComponentMap`.
///
/// Stores differences from a prototype:
/// - Components that are added or overridden (`Set`)
/// - Components that are explicitly removed (`Removed`)
#[derive(Debug, Default)]
pub struct DataComponentPatch {
    entries: HashMap<Identifier, ComponentPatchEntry>,
}

impl PartialEq for DataComponentPatch {
    fn eq(&self, other: &Self) -> bool {
        if self.entries.len() != other.entries.len() {
            return false;
        }
        self.entries
            .iter()
            .all(|(k, v)| other.entries.get(k).is_some_and(|ov| v == ov))
    }
}

impl DataComponentPatch {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn set<T: 'static + ComponentValue>(&mut self, component: DataComponentType<T>, value: T) {
        self.entries.insert(
            component.key.clone(),
            ComponentPatchEntry::Set(Box::new(value)),
        );
    }

    pub fn remove<T>(&mut self, component: DataComponentType<T>) {
        self.entries
            .insert(component.key.clone(), ComponentPatchEntry::Removed);
    }

    pub fn clear<T>(&mut self, component: DataComponentType<T>) {
        self.entries.remove(&component.key);
    }

    #[must_use]
    pub fn get_entry(&self, key: &Identifier) -> Option<&ComponentPatchEntry> {
        self.entries.get(key)
    }

    #[must_use]
    pub fn is_removed(&self, key: &Identifier) -> bool {
        matches!(self.entries.get(key), Some(ComponentPatchEntry::Removed))
    }

    #[must_use]
    pub fn count_set(&self) -> usize {
        self.entries
            .values()
            .filter(|e| matches!(e, ComponentPatchEntry::Set(_)))
            .count()
    }

    #[must_use]
    pub fn count_removed(&self) -> usize {
        self.entries
            .values()
            .filter(|e| matches!(e, ComponentPatchEntry::Removed))
            .count()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &ComponentPatchEntry)> {
        self.entries.iter()
    }

    pub fn iter_removed(&self) -> impl Iterator<Item = &Identifier> {
        self.entries.iter().filter_map(|(k, v)| {
            if matches!(v, ComponentPatchEntry::Removed) {
                Some(k)
            } else {
                None
            }
        })
    }
}

fn get_effective_value<'a>(
    prototype: &'a DataComponentMap,
    patch: &'a DataComponentPatch,
    key: &Identifier,
) -> Option<&'a dyn ComponentValue> {
    match patch.get_entry(key) {
        Some(ComponentPatchEntry::Set(v)) => Some(v.as_ref()),
        Some(ComponentPatchEntry::Removed) => None,
        None => prototype.get_raw(key),
    }
}

/// Compares effective components between two (prototype, patch) pairs.
/// Returns true if all effective component values are equal.
#[must_use]
pub fn effective_components_equal(
    proto_a: &DataComponentMap,
    patch_a: &DataComponentPatch,
    proto_b: &DataComponentMap,
    patch_b: &DataComponentPatch,
) -> bool {
    let mut all_keys = std::collections::HashSet::new();

    for key in proto_a.keys() {
        if !patch_a.is_removed(key) {
            all_keys.insert(key);
        }
    }
    for (key, entry) in patch_a.iter() {
        if matches!(entry, ComponentPatchEntry::Set(_)) {
            all_keys.insert(key);
        }
    }
    for key in proto_b.keys() {
        if !patch_b.is_removed(key) {
            all_keys.insert(key);
        }
    }
    for (key, entry) in patch_b.iter() {
        if matches!(entry, ComponentPatchEntry::Set(_)) {
            all_keys.insert(key);
        }
    }
    for key in all_keys {
        let val_a = get_effective_value(proto_a, patch_a, key);
        let val_b = get_effective_value(proto_b, patch_b, key);

        match (val_a, val_b) {
            (Some(a), Some(b)) => {
                if !a.eq_value(b) {
                    return false;
                }
            }
            (None, None) => {}
            _ => return false,
        }
    }

    true
}
