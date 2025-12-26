//! Container set slot packet.

use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::play::C_CONTAINER_SET_SLOT;

use super::slot_data::SlotData;

/// Updates a single slot in a container.
#[derive(WriteTo, ClientPacket, Clone, Debug)]
#[packet_id(Play = C_CONTAINER_SET_SLOT)]
pub struct CContainerSetSlot {
    /// The container ID (0 = player inventory, -1 = cursor, -2 = player inventory update).
    #[write(as = Byte)]
    pub container_id: i8,
    /// State ID for synchronization.
    #[write(as = VarInt)]
    pub state_id: i32,
    /// The slot index being updated.
    pub slot: i16,
    /// The new slot contents.
    pub slot_data: SlotData,
}

impl CContainerSetSlot {
    /// Creates a new container set slot packet.
    #[must_use]
    pub fn new(container_id: i8, state_id: i32, slot: i16, slot_data: SlotData) -> Self {
        Self {
            container_id,
            state_id,
            slot,
            slot_data,
        }
    }

    /// Creates a packet to update the cursor item.
    #[must_use]
    pub fn cursor(state_id: i32, slot_data: SlotData) -> Self {
        Self::new(-1, state_id, -1, slot_data)
    }
}

