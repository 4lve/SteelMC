//! Container set content packet.

use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::play::C_CONTAINER_SET_CONTENT;

use super::slot_data::SlotData;

/// Sends the entire contents of a container to the client.
#[derive(WriteTo, ClientPacket, Clone, Debug)]
#[packet_id(Play = C_CONTAINER_SET_CONTENT)]
pub struct CContainerSetContent {
    /// The container ID (0 = player inventory).
    #[write(as = Byte)]
    pub container_id: i8,
    /// State ID for synchronization.
    #[write(as = VarInt)]
    pub state_id: i32,
    /// All slot contents.
    #[write(as = Prefixed(VarInt))]
    pub slots: Vec<SlotData>,
    /// The item on the cursor.
    pub carried: SlotData,
}

impl CContainerSetContent {
    /// Creates a new container set content packet.
    #[must_use]
    pub fn new(container_id: i8, state_id: i32, slots: Vec<SlotData>, carried: SlotData) -> Self {
        Self {
            container_id,
            state_id,
            slots,
            carried,
        }
    }
}
