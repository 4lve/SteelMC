//! Clientbound remove entities packet (despawns entities on the client)

use steel_macros::ClientPacket;
use steel_registry::packets::play::C_REMOVE_ENTITIES;
use steel_utils::codec::VarInt;
use steel_utils::serial::WriteTo;

/// Removes/despawns entities on the client
#[derive(ClientPacket, Debug, Clone)]
#[packet_id(Play = C_REMOVE_ENTITIES)]
pub struct CRemoveEntities {
    /// List of entity IDs to remove
    pub entity_ids: Vec<i32>,
}

impl WriteTo for CRemoveEntities {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        VarInt(self.entity_ids.len() as i32).write(writer)?;
        for entity_id in &self.entity_ids {
            VarInt(*entity_id).write(writer)?;
        }
        Ok(())
    }
}
