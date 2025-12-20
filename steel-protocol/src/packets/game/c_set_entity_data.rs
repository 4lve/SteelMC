//! Clientbound set entity data packet (updates entity metadata)

use steel_macros::ClientPacket;
use steel_registry::packets::play::C_SET_ENTITY_DATA;
use steel_utils::codec::VarInt;
use steel_utils::serial::WriteTo;

/// Entity data value (opaque wrapper for serialization)
#[derive(Debug, Clone)]
pub struct EntityDataEntry {
    pub field_id: u8,
    pub serializer_id: u8,
    pub value_bytes: Vec<u8>,
}

/// Updates entity metadata on the client
#[derive(ClientPacket, Debug, Clone)]
#[packet_id(Play = C_SET_ENTITY_DATA)]
pub struct CSetEntityData {
    /// The entity ID to update
    pub entity_id: i32,
    /// The metadata entries
    pub metadata: Vec<EntityDataEntry>,
}

impl WriteTo for CSetEntityData {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        VarInt(self.entity_id).write(writer)?;

        // Write each metadata entry
        for entry in &self.metadata {
            entry.field_id.write(writer)?;
            entry.serializer_id.write(writer)?;
            writer.write_all(&entry.value_bytes)?;
        }

        // Write end marker (0xFF)
        0xFFu8.write(writer)?;
        Ok(())
    }
}
