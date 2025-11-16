pub mod behaviour;
pub mod properties;

use std::collections::HashMap;

use crate::RegistryExt;
use crate::blocks::behaviour::BlockBehaviourProperties;
use crate::blocks::properties::{DynProperty, Property};

#[derive(Debug)]
pub struct Block {
    pub key: Identifier,
    pub behaviour: BlockBehaviourProperties,
    pub properties: &'static [&'static dyn DynProperty],
    pub default_state_offset: u16,
}

impl Block {
    pub const fn new(
        key: Identifier,
        behaviour: BlockBehaviourProperties,
        properties: &'static [&'static dyn DynProperty],
    ) -> Self {
        Self {
            key,
            behaviour,
            properties,
            default_state_offset: 0,
        }
    }

    /// Sets the default state offset for this block.
    /// The offset is relative to the block's base state ID.
    ///
    /// For easier usage, consider using `with_default_state_from_indices` or the
    /// `default_state!` macro instead of calculating the offset manually.
    ///
    /// # Example
    /// ```ignore
    /// const REPEATER: Block = Block::new("repeater", props, &[...])
    ///     .with_default_state(4);
    /// ```
    pub(crate) const fn with_default_state(mut self, offset: u16) -> Self {
        self.default_state_offset = offset;

        self
    }

    /// Const helper to calculate state offset from property indices and counts
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn calculate_offset(property_indices: &[usize], property_counts: &[usize]) -> u16 {
        let mut offset = 0u16;
        let mut multiplier = 1u16;
        let mut i = 0;

        while i < property_indices.len() {
            offset += property_indices[i] as u16 * multiplier;
            multiplier *= property_counts[i] as u16;
            i += 1;
        }

        offset
    }
}

pub type BlockRef = &'static Block;

// The central registry for all blocks.
pub struct BlockRegistry {
    blocks_by_id: Vec<BlockRef>,
    blocks_by_key: HashMap<Identifier, usize>,
    tags: HashMap<Identifier, Vec<BlockRef>>,
    allows_registering: bool,
    pub state_to_block_lookup: Vec<BlockRef>,
    /// Maps state IDs to block IDs (parallel to `state_to_block_lookup` for O(1) lookup)
    pub state_to_block_id: Vec<usize>,
    /// Maps block IDs to their base state ID
    pub block_to_base_state: Vec<u16>,
    /// The next state ID to be allocated
    pub next_state_id: u16,
}

impl Default for BlockRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockRegistry {
    // Creates a new, empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            blocks_by_id: Vec::new(),
            blocks_by_key: HashMap::new(),
            tags: HashMap::new(),
            allows_registering: true,
            state_to_block_lookup: Vec::new(),
            state_to_block_id: Vec::new(),
            block_to_base_state: Vec::new(),
            next_state_id: 0,
        }
    }

    // Registers a new block.
    pub fn register(&mut self, block: BlockRef) -> usize {
        assert!(
            self.allows_registering,
            "Cannot register blocks after the registry has been frozen"
        );

        let id = self.blocks_by_id.len();
        let base_state_id = self.next_state_id;

        self.blocks_by_key.insert(block.key.clone(), id);
        self.blocks_by_id.push(block);
        self.block_to_base_state.push(base_state_id);

        let mut state_count = 1;
        for property in block.properties {
            state_count *= property.get_possible_values().len();
        }

        for _ in 0..state_count {
            self.state_to_block_lookup.push(block);
            self.state_to_block_id.push(id);
        }

        self.next_state_id += u16::try_from(state_count).unwrap();

        id
    }

    #[must_use]
    pub fn get_base_state_id(&self, block: BlockRef) -> BlockStateId {
        BlockStateId(self.block_to_base_state[*self.get_id(block)])
    }

    /// Gets the default state ID for a block (base state + default offset)
    #[must_use]
    pub fn get_default_state_id(&self, block: BlockRef) -> BlockStateId {
        let base = self.block_to_base_state[*self.get_id(block)];
        BlockStateId(base + block.default_state_offset)
    }

    // Retrieves a block by its ID.
    #[must_use]
    pub fn by_id(&self, id: usize) -> Option<BlockRef> {
        self.blocks_by_id.get(id).copied()
    }

    #[must_use]
    pub fn get_id(&self, block: BlockRef) -> &usize {
        self.blocks_by_key.get(&block.key).expect("Block not found")
    }

    #[must_use]
    pub fn by_state_id(&self, state_id: BlockStateId) -> Option<BlockRef> {
        self.state_to_block_lookup.get(state_id.0 as usize).copied()
    }

    // Retrieves a block by its name.
    #[must_use]
    pub fn by_key(&self, key: &Identifier) -> Option<BlockRef> {
        self.blocks_by_key.get(key).and_then(|id| self.by_id(*id))
    }

    #[must_use]
    pub fn get_properties(&self, id: BlockStateId) -> Vec<(&str, &str)> {
        let block = self.by_state_id(id).expect("Invalid state ID");

        // If block has no properties, return empty vec
        if block.properties.is_empty() {
            return Vec::new();
        }

        // Get the base state ID for this block (O(1) lookup)
        let block_id = self.state_to_block_id[id.0 as usize];
        let base_state_id = self.block_to_base_state[block_id];

        // Calculate the relative state index
        let relative_index = id.0 - base_state_id;

        // Decode the property indices from the relative state index
        let mut index = relative_index;
        let mut property_values = Vec::with_capacity(block.properties.len());

        for prop in block.properties {
            let count = u16::try_from(prop.get_possible_values().len()).unwrap();
            let current_index = (index % count) as usize;

            let possible_values = prop.get_possible_values();
            property_values.push((prop.get_name(), possible_values[current_index]));

            index /= count;
        }

        property_values
    }

    // Panics if that property isn't supposed to be on this block.
    pub fn get_property<T, P: Property<T>>(&self, id: BlockStateId, property: &P) -> T {
        let block = self.by_state_id(id).expect("Invalid state ID");

        // Find the property index in the block's property list
        let property_index = block
            .properties
            .iter()
            .position(|prop| prop.get_name() == property.as_dyn().get_name())
            .expect("Property not found on this block");

        // Get the base state ID for this block (O(1) lookup)
        let block_id = self.state_to_block_id[id.0 as usize];
        let base_state_id = self.block_to_base_state[block_id];

        // Calculate the relative state index
        let relative_index = id.0 - base_state_id;

        // Decode the property indices from the relative state index
        let mut index = relative_index;
        let mut property_value_index = 0;

        for (i, prop) in block.properties.iter().enumerate() {
            let count = u16::try_from(prop.get_possible_values().len()).unwrap();
            let current_index = (index % count) as usize;

            if i == property_index {
                property_value_index = current_index;
            }

            index /= count;
        }

        // Convert the index back to the actual value
        property.value_from_index(property_value_index)
    }

    // Panics if that property isn't supposed to be on this block.
    pub fn set_property<T, P: Property<T>>(
        &self,
        id: BlockStateId,
        property: &P,
        value: T,
    ) -> BlockStateId {
        let block = self.by_state_id(id).expect("Invalid state ID");

        // Find the property index in the block's property list
        let property_index = block
            .properties
            .iter()
            .position(|prop| prop.get_name() == property.as_dyn().get_name())
            .unwrap_or_else(|| {
                panic!(
                    "Property {} not found on block {}",
                    property.as_dyn().get_name(),
                    block.key
                )
            });

        // Get the base state ID for this block (O(1) lookup)
        let block_id = self.state_to_block_id[id.0 as usize];
        let base_state_id = self.block_to_base_state[block_id];

        // Calculate the relative state index
        let relative_index = id.0 - base_state_id;

        // Decode all property indices from the relative state index
        let mut index = relative_index;
        let mut property_indices = Vec::with_capacity(block.properties.len());

        for prop in block.properties {
            let count = u16::try_from(prop.get_possible_values().len()).unwrap();
            property_indices.push((index % count) as usize);
            index /= count;
        }

        // Update the specific property's index
        let new_value_index = property.get_internal_index(&value);
        property_indices[property_index] = new_value_index;

        // Re-encode the property indices back to a state ID
        let (new_relative_index, _) = property_indices.iter().zip(block.properties.iter()).fold(
            (0u16, 1u16),
            |(current_index, multiplier), (&value_idx, prop)| {
                let count = u16::try_from(prop.get_possible_values().len()).unwrap();
                (
                    current_index + u16::try_from(value_idx).unwrap() * multiplier,
                    multiplier * count,
                )
            },
        );

        BlockStateId(base_state_id + new_relative_index)
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, BlockRef)> + '_ {
        self.blocks_by_id
            .iter()
            .enumerate()
            .map(|(id, &block)| (id, block))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.blocks_by_id.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.blocks_by_id.is_empty()
    }

    // Tag-related methods

    /// Registers a tag with a list of block keys.
    /// Block keys that don't exist in the registry are silently skipped.
    pub fn register_tag(&mut self, tag: Identifier, block_keys: &[&'static str]) {
        assert!(
            self.allows_registering,
            "Cannot register tags after registry has been frozen"
        );

        let blocks: Vec<BlockRef> = block_keys
            .iter()
            .filter_map(|key| self.by_key(&Identifier::vanilla_static(key)))
            .collect();

        self.tags.insert(tag, blocks);
    }

    /// Checks if a block is in a given tag.
    #[must_use]
    pub fn is_in_tag(&self, block: BlockRef, tag: &Identifier) -> bool {
        self.tags.get(tag).is_some_and(|blocks| {
            blocks
                .iter()
                .any(|&b| std::ptr::eq(std::ptr::from_ref(b), std::ptr::from_ref(block)))
        })
    }

    /// Gets all blocks in a tag.
    #[must_use]
    pub fn get_tag(&self, tag: &Identifier) -> Option<&[BlockRef]> {
        self.tags.get(tag).map(std::vec::Vec::as_slice)
    }

    /// Iterates over all blocks in a tag.
    pub fn iter_tag(&self, tag: &Identifier) -> impl Iterator<Item = BlockRef> + '_ {
        self.tags
            .get(tag)
            .map(|v| v.iter().copied())
            .into_iter()
            .flatten()
    }

    /// Gets all tag keys.
    pub fn tag_keys(&self) -> impl Iterator<Item = &Identifier> + '_ {
        self.tags.keys()
    }
}

impl RegistryExt for BlockRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

/// Macro to generate offset calculation from property values in all positions.
///
/// Takes property objects and their values, automatically converts to indices.
/// All properties must be specified in order.
///
/// # Note
/// For boolean properties, use `.index_of(value)` to handle the inverted encoding
/// (true=0, false=1 for Java compatibility).
///
/// # Example
/// ```ignore
/// use steel_registry::{offset, properties::{BlockStateProperties as Props, RedstoneSide}};
///
/// const WIRE: Block = Block::new("wire", behaviour, PROPS)
///     .with_default_state(offset!(
///         Props::EAST_REDSTONE => RedstoneSide::Up,
///         Props::NORTH_REDSTONE => RedstoneSide::None,
///         Props::POWER => 10,
///         Props::ATTACHED => Props::ATTACHED.index_of(false)  // Bools need .index_of()
///     ));
/// ```
#[macro_export]
macro_rules! offset {
    ($($prop:expr => $value:expr),* $(,)?) => {{
        const INDICES: &[usize] = &[$($value as usize),*];
        const COUNTS: &[usize] = &[$($prop.value_count()),*];
        $crate::blocks::Block::calculate_offset(INDICES, COUNTS)
    }};
}

/// Re-export for easier access
pub use offset;
use steel_utils::{BlockStateId, Identifier};
