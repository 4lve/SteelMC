//! Network slot data for inventory packets.
//!
//! This is a simplified representation of ItemStack for network serialization.
//! Full component serialization will be added later.

use std::io::{Read, Result, Write};

use steel_registry::{Registry, item_stack::ItemStack};
use steel_utils::{
    codec::VarInt,
    serial::{ReadFrom, WriteTo},
};

/// A slot's contents for network transmission.
///
/// In Minecraft's protocol, a slot is serialized as:
/// - count: VarInt (0 = empty slot)
/// - if count > 0:
///   - item_id: VarInt
///   - components_to_add_count: VarInt
///   - components_to_remove_count: VarInt
///   - (component data follows)
///
/// For now, we only support empty slots and basic items without components.
#[derive(Debug, Clone, Default)]
pub struct SlotData {
    /// The item ID (registry ID, not raw item ID).
    pub item_id: Option<i32>,
    /// The item count.
    pub count: i32,
}

impl SlotData {
    /// Creates an empty slot.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            item_id: None,
            count: 0,
        }
    }

    /// Creates a slot with an item.
    #[must_use]
    pub const fn new(item_id: i32, count: i32) -> Self {
        Self {
            item_id: Some(item_id),
            count,
        }
    }

    /// Returns whether this slot is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count <= 0 || self.item_id.is_none()
    }

    /// Creates slot data from an item stack.
    #[must_use]
    pub fn from_item_stack(item: &ItemStack, registry: &Registry) -> Self {
        if item.is_empty() {
            Self::empty()
        } else {
            let item_id = *registry.items.get_id(item.item());
            Self {
                item_id: Some(item_id as i32),
                count: item.count(),
            }
        }
    }

    /// Converts slot data to an item stack.
    #[must_use]
    pub fn to_item_stack(&self, registry: &Registry) -> ItemStack {
        if self.is_empty() {
            ItemStack::empty()
        } else if let Some(item_id) = self.item_id {
            if let Some(item) = registry.items.by_id(item_id as usize) {
                ItemStack::with_count(item, self.count)
            } else {
                ItemStack::empty()
            }
        } else {
            ItemStack::empty()
        }
    }
}

impl WriteTo for SlotData {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        if self.is_empty() {
            VarInt(0).write(writer)?;
        } else {
            VarInt(self.count).write(writer)?;
            VarInt(self.item_id.unwrap_or(0)).write(writer)?;
            // No components for now
            VarInt(0).write(writer)?; // components_to_add
            VarInt(0).write(writer)?; // components_to_remove
        }
        Ok(())
    }
}

impl ReadFrom for SlotData {
    fn read(data: &mut impl Read) -> Result<Self> {
        let count = VarInt::read(data)?.0;
        if count <= 0 {
            return Ok(Self::empty());
        }

        let item_id = VarInt::read(data)?.0;
        let _components_to_add = VarInt::read(data)?.0;
        let _components_to_remove = VarInt::read(data)?.0;

        // TODO: Read component data when implemented

        Ok(Self {
            item_id: Some(item_id),
            count,
        })
    }
}
