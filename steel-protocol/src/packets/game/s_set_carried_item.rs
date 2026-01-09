use steel_macros::{ReadFrom, ServerPacket};

#[derive(ServerPacket, ReadFrom, Clone, Debug)]
pub struct SSetCarriedItem {
    pub slot: u16,
}
