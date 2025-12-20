//! Clientbound remove player info packet (removes players from tab list)

use steel_macros::ClientPacket;
use steel_registry::packets::play::C_PLAYER_INFO_REMOVE;
use steel_utils::codec::VarInt;
use steel_utils::serial::WriteTo;
use uuid::Uuid;

/// Removes players from the client's tab list
#[derive(ClientPacket, Debug, Clone)]
#[packet_id(Play = C_PLAYER_INFO_REMOVE)]
pub struct CRemovePlayerInfo {
    /// UUIDs of players to remove
    pub uuids: Vec<Uuid>,
}

impl WriteTo for CRemovePlayerInfo {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        VarInt(self.uuids.len() as i32).write(writer)?;
        for uuid in &self.uuids {
            uuid.write(writer)?;
        }
        Ok(())
    }
}
