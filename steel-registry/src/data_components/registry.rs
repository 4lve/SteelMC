use rustc_hash::FxHashMap;
use simdnbt::{
    FromNbtTag, ToNbtTag, borrow::NbtTag as BorrowedNbtTag, owned::NbtTag as OwnedNbtTag,
};
use std::{
    any::Any,
    fmt::Debug,
    io::{Cursor, Result, Write},
    marker::PhantomData,
};

use steel_utils::{
    Identifier,
    codec::VarInt,
    hash::HashComponent,
    serial::{ReadFrom, WriteTo},
    types::Todo,
};

use crate::{
    RegistryExt,
    data_components::vanilla_components::{
        ATTRIBUTE_MODIFIERS, BREAK_SOUND, ENCHANTMENTS, LORE, MAX_STACK_SIZE, RARITY, REPAIR_COST,
        TOOLTIP_DISPLAY,
    },
};

/// Type alias for a component reader function (network format).
/// Takes a cursor and returns a boxed `ComponentValue`.
type ComponentReader = fn(&mut Cursor<&[u8]>) -> Result<Box<dyn ComponentValue>>;

/// Type alias for a component NBT reader function (persistent storage).
/// Takes a borrowed NBT tag and returns a boxed `ComponentValue`.
type ComponentNbtReader = fn(BorrowedNbtTag) -> Option<Box<dyn ComponentValue>>;

pub trait ComponentValue: Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn clone_boxed(&self) -> Box<dyn ComponentValue>;
    fn eq_value(&self, other: &dyn ComponentValue) -> bool;
    /// Writes this component value to the network stream.
    fn write_network(&self, writer: &mut Vec<u8>) -> Result<()>;
    /// Computes the hash of this component value for validation.
    fn compute_hash(&self) -> i32;
    /// Converts this component value to an NBT tag for persistent storage.
    fn to_nbt(&self) -> OwnedNbtTag;
}

impl<
    T: 'static
        + Send
        + Sync
        + Debug
        + Clone
        + PartialEq
        + WriteTo
        + ReadFrom
        + HashComponent
        + ToNbtTag
        + FromNbtTag,
> ComponentValue for T
{
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

    fn write_network(&self, writer: &mut Vec<u8>) -> Result<()> {
        self.write(writer)
    }

    fn compute_hash(&self) -> i32 {
        HashComponent::compute_hash(self)
    }

    fn to_nbt(&self) -> OwnedNbtTag {
        self.clone().to_nbt_tag()
    }
}

impl ToNbtTag for Box<dyn ComponentValue> {
    fn to_nbt_tag(self) -> OwnedNbtTag {
        self.to_nbt()
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
    /// Map from component key to network ID
    components_by_key: FxHashMap<Identifier, usize>,
    /// Ordered list of component keys (index = network ID)
    components_by_id: Vec<Identifier>,
    /// Ordered list of network reader functions (index = network ID)
    readers_by_id: Vec<ComponentReader>,
    /// Ordered list of NBT reader functions (index = network ID)
    nbt_readers_by_id: Vec<ComponentNbtReader>,
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
            components_by_key: FxHashMap::default(),
            components_by_id: Vec::new(),
            readers_by_id: Vec::new(),
            nbt_readers_by_id: Vec::new(),
            allows_registering: true,
        }
    }

    pub fn register<T: 'static + ComponentValue + ReadFrom + FromNbtTag>(
        &mut self,
        component: DataComponentType<T>,
    ) {
        assert!(
            self.allows_registering,
            "Cannot register data components after the registry has been frozen"
        );

        let id = self.components_by_id.len();
        self.components_by_key.insert(component.key.clone(), id);
        self.components_by_id.push(component.key);

        // Create a reader function that deserializes T from network format
        fn read_component<T: 'static + ComponentValue + ReadFrom>(
            data: &mut Cursor<&[u8]>,
        ) -> Result<Box<dyn ComponentValue>> {
            let value = T::read(data)?;
            Ok(Box::new(value))
        }

        // Create a reader function that deserializes T from NBT format
        fn read_nbt_component<T: 'static + ComponentValue + FromNbtTag>(
            tag: BorrowedNbtTag,
        ) -> Option<Box<dyn ComponentValue>> {
            let value = T::from_nbt_tag(tag)?;
            Some(Box::new(value))
        }

        self.readers_by_id.push(read_component::<T>);
        self.nbt_readers_by_id.push(read_nbt_component::<T>);
    }

    #[must_use]
    pub fn get_id<T: 'static>(&self, component: DataComponentType<T>) -> Option<usize> {
        self.components_by_key.get(&component.key).copied()
    }

    #[must_use]
    pub fn get_id_by_key(&self, key: &Identifier) -> Option<usize> {
        self.components_by_key.get(key).copied()
    }

    /// Gets the component key by its network ID.
    #[must_use]
    pub fn get_key_by_id(&self, id: usize) -> Option<&Identifier> {
        self.components_by_id.get(id)
    }

    /// Gets the network reader function for a component by its network ID.
    #[must_use]
    pub fn get_reader_by_id(&self, id: usize) -> Option<ComponentReader> {
        self.readers_by_id.get(id).copied()
    }

    /// Gets the NBT reader function for a component by its network ID.
    #[must_use]
    pub fn get_nbt_reader_by_id(&self, id: usize) -> Option<ComponentNbtReader> {
        self.nbt_readers_by_id.get(id).copied()
    }

    /// Gets the NBT reader function for a component by its key.
    #[must_use]
    pub fn get_nbt_reader_by_key(&self, key: &Identifier) -> Option<ComponentNbtReader> {
        let id = self.get_id_by_key(key)?;
        self.get_nbt_reader_by_id(id)
    }

    /// Returns the number of registered components.
    #[must_use]
    pub fn len(&self) -> usize {
        self.components_by_id.len()
    }

    /// Returns true if no components are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.components_by_id.is_empty()
    }
}

impl RegistryExt for DataComponentRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

#[derive(Debug)]
pub struct DataComponentMap {
    map: FxHashMap<Identifier, Box<dyn ComponentValue>>,
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
            map: FxHashMap::default(),
        }
    }

    #[must_use]
    pub fn common_item_components() -> Self {
        let mut map = FxHashMap::default();
        map.insert(
            MAX_STACK_SIZE.key.clone(),
            Box::new(64i32) as Box<dyn ComponentValue>,
        );
        map.insert(LORE.key.clone(), Box::new(Todo) as Box<dyn ComponentValue>);
        map.insert(
            ENCHANTMENTS.key.clone(),
            Box::new(Todo) as Box<dyn ComponentValue>,
        );
        map.insert(
            REPAIR_COST.key.clone(),
            Box::new(0i32) as Box<dyn ComponentValue>,
        );
        map.insert(
            ATTRIBUTE_MODIFIERS.key.clone(),
            Box::new(Todo) as Box<dyn ComponentValue>,
        );
        map.insert(
            RARITY.key.clone(),
            Box::new(Todo) as Box<dyn ComponentValue>,
        );
        map.insert(
            BREAK_SOUND.key.clone(),
            Box::new(Todo) as Box<dyn ComponentValue>,
        );
        map.insert(
            TOOLTIP_DISPLAY.key.clone(),
            Box::new(Todo) as Box<dyn ComponentValue>,
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

impl Clone for ComponentPatchEntry {
    fn clone(&self) -> Self {
        match self {
            Self::Set(v) => Self::Set(v.clone_boxed()),
            Self::Removed => Self::Removed,
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
    entries: FxHashMap<Identifier, ComponentPatchEntry>,
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

impl Clone for DataComponentPatch {
    fn clone(&self) -> Self {
        Self {
            entries: self
                .entries
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }
}

impl DataComponentPatch {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: FxHashMap::default(),
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

pub fn component_try_into<T: 'static>(
    value: &dyn ComponentValue,
    _component: DataComponentType<T>,
) -> Option<&T> {
    value.as_any().downcast_ref::<T>()
}

impl WriteTo for DataComponentPatch {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        use crate::REGISTRY;

        // Format: VarInt addedCount, VarInt removedCount, then added entries, then removed entries
        // Collect added and removed entries
        let mut added: Vec<(&Identifier, &Box<dyn ComponentValue>)> = Vec::new();
        let mut removed: Vec<&Identifier> = Vec::new();

        for (key, entry) in &self.entries {
            match entry {
                ComponentPatchEntry::Set(value) => added.push((key, value)),
                ComponentPatchEntry::Removed => removed.push(key),
            }
        }

        VarInt(added.len() as i32).write(writer)?;
        VarInt(removed.len() as i32).write(writer)?;

        // Write added components: VarInt type_id, then component data
        for (key, value) in added {
            let id = REGISTRY
                .data_components
                .get_id_by_key(key)
                .ok_or_else(|| std::io::Error::other(format!("Unknown component key: {key:?}")))?;
            VarInt(id as i32).write(writer)?;

            // Write the component value
            let mut buf = Vec::new();
            value.write_network(&mut buf)?;
            writer.write_all(&buf)?;
        }

        // Write removed component IDs
        for key in removed {
            let id = REGISTRY
                .data_components
                .get_id_by_key(key)
                .ok_or_else(|| std::io::Error::other(format!("Unknown component key: {key:?}")))?;
            VarInt(id as i32).write(writer)?;
        }

        Ok(())
    }
}

impl ReadFrom for DataComponentPatch {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        use crate::REGISTRY;

        // Format: VarInt addedCount, VarInt removedCount, then added entries, then removed entries
        let added_count = VarInt::read(data)?.0 as usize;
        let removed_count = VarInt::read(data)?.0 as usize;

        log::info!("Reading DataComponentPatch: added={added_count}, removed={removed_count}");

        let mut patch = Self::new();

        // Read added components
        for i in 0..added_count {
            let pos_before = data.position();
            let type_id = VarInt::read(data)?.0 as usize;

            let key = REGISTRY
                .data_components
                .get_key_by_id(type_id)
                .ok_or_else(|| {
                    std::io::Error::other(format!("Unknown component type ID: {type_id}"))
                })?
                .clone();

            log::info!("  [{i}] Reading component {key} (id={type_id}) at pos {pos_before}");

            // Try to get a reader for this component type
            if let Some(reader) = REGISTRY.data_components.get_reader_by_id(type_id) {
                let value = reader(data).map_err(|e| {
                    log::error!("    Failed to read component {key}: {e}");
                    e
                })?;
                let pos_after = data.position();
                log::info!("    Read {} bytes for {key}", pos_after - pos_before);
                patch.entries.insert(key, ComponentPatchEntry::Set(value));
            } else {
                // No reader registered for this component - we can't skip it properly
                // since we don't know its size
                return Err(std::io::Error::other(format!(
                    "No reader registered for component: {key}"
                )));
            }
        }

        // Read removed component IDs
        for _ in 0..removed_count {
            let type_id = VarInt::read(data)?.0 as usize;

            let key = REGISTRY
                .data_components
                .get_key_by_id(type_id)
                .ok_or_else(|| {
                    std::io::Error::other(format!("Unknown component type ID: {type_id}"))
                })?
                .clone();

            patch.entries.insert(key, ComponentPatchEntry::Removed);
        }

        Ok(patch)
    }
}

// ==================== NBT Serialization ====================

use simdnbt::owned::NbtCompound;

impl ToNbtTag for DataComponentPatch {
    /// Converts this component patch to an NBT tag for persistent storage.
    ///
    /// Format (matching vanilla Minecraft):
    /// ```text
    /// {
    ///     "minecraft:damage": 10,
    ///     "!minecraft:unbreakable": {}  // "!" prefix means removed
    /// }
    /// ```
    fn to_nbt_tag(self) -> OwnedNbtTag {
        let mut compound = NbtCompound::new();

        for (key, entry) in self.entries {
            match entry {
                ComponentPatchEntry::Set(value) => {
                    compound.insert(key.to_string(), value.to_nbt());
                }
                ComponentPatchEntry::Removed => {
                    // Removed components use "!" prefix and empty compound as value
                    compound.insert(format!("!{key}"), NbtCompound::new());
                }
            }
        }

        OwnedNbtTag::Compound(compound)
    }
}

impl FromNbtTag for DataComponentPatch {
    /// Parses a component patch from an NBT tag.
    ///
    /// Format:
    /// ```text
    /// {
    ///     "minecraft:damage": 10,
    ///     "!minecraft:unbreakable": {}  // "!" prefix means removed
    /// }
    /// ```
    fn from_nbt_tag(tag: BorrowedNbtTag) -> Option<Self> {
        use crate::REGISTRY;

        let compound = tag.compound()?;
        let mut patch = Self::new();

        for (key, value) in compound.iter() {
            let key_str = key.to_str();

            if let Some(stripped) = key_str.strip_prefix('!') {
                // Removed component
                if let Ok(id) = stripped.parse::<Identifier>() {
                    patch.entries.insert(id, ComponentPatchEntry::Removed);
                }
            } else {
                // Set component - use registry to deserialize
                if let Ok(id) = key_str.parse::<Identifier>()
                    && let Some(reader) = REGISTRY.data_components.get_nbt_reader_by_key(&id)
                    && let Some(component_value) = reader(value)
                {
                    patch
                        .entries
                        .insert(id, ComponentPatchEntry::Set(component_value));
                }
            }
        }

        Some(patch)
    }
}
