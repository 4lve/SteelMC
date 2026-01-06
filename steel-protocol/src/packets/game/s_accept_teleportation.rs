use steel_macros::{ReadFrom, ServerPacket};
use steel_utils::codec::VarInt;

/// Client acknowledges a teleport sent by the server
#[derive(ReadFrom, Clone, Debug, ServerPacket)]
pub struct SAcceptTeleportation {
    /// The teleport ID being acknowledged
    pub teleport_id: VarInt,
}
