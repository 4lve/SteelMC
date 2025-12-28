//! Set creative mode slot packet.

use std::io::{Read, Result};

use steel_macros::ServerPacket;
use steel_utils::serial::ReadFrom;

use super::slot_data::SlotData;

/// Client sets a slot in creative mode.
///
/// In creative mode, players can directly set items in their inventory.
/// A negative slot number means the player wants to drop the item.
#[derive(ServerPacket, Debug, Clone)]
pub struct SSetCreativeModeSlot {
    /// The slot index. Negative values mean drop the item.
    /// Valid slots for setting are 1-45 (crafting result excluded, but includes
    /// crafting grid, armor, main inventory, hotbar, and offhand).
    pub slot: i16,
    /// The item to set in the slot.
    pub item: SlotData,
}

impl ReadFrom for SSetCreativeModeSlot {
    fn read(data: &mut impl Read) -> Result<Self> {
        let slot = i16::read(data)?;
        let item = SlotData::read(data)?;
        Ok(Self { slot, item })
    }
}

impl SSetCreativeModeSlot {
    /// Returns whether this packet represents a drop action.
    #[must_use]
    pub fn is_drop(&self) -> bool {
        self.slot < 0
    }

    /// Returns whether the slot is valid for setting items (1-45).
    #[must_use]
    pub fn is_valid_slot(&self) -> bool {
        self.slot >= 1 && self.slot <= 45
    }
}
