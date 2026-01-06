//! Clientbound rotate head packet (updates entity head yaw)

use steel_macros::ClientPacket;
use steel_registry::packets::play::C_ROTATE_HEAD;
use steel_utils::codec::VarInt;
use steel_utils::serial::WriteTo;

/// Updates an entity's head yaw rotation
#[derive(ClientPacket, Debug, Clone)]
#[packet_id(Play = C_ROTATE_HEAD)]
pub struct CRotateHead {
    /// The entity ID
    pub entity_id: VarInt,
    /// Head yaw (256 = 360 degrees)
    pub head_yaw: i8,
}

impl WriteTo for CRotateHead {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.entity_id.write(writer)?;
        self.head_yaw.write(writer)
    }
}
