//! Clientbound add entity packet (spawns an entity on the client)

use steel_macros::ClientPacket;
use steel_registry::packets::play::C_ADD_ENTITY;
use steel_utils::codec::VarInt;
use steel_utils::serial::WriteTo;
use uuid::Uuid;

/// Spawns an entity on the client
#[derive(ClientPacket, Debug, Clone)]
#[packet_id(Play = C_ADD_ENTITY)]
pub struct CAddEntity {
    /// The entity's unique ID
    pub entity_id: i32,
    /// The entity's UUID
    pub uuid: Uuid,
    /// The entity type ID
    pub entity_type: VarInt,
    /// Position X
    pub x: f64,
    /// Position Y
    pub y: f64,
    /// Position Z
    pub z: f64,
    /// Pitch (rotation around X axis)
    pub pitch: i8,
    /// Yaw (rotation around Y axis)
    pub yaw: i8,
    /// Head yaw
    pub head_yaw: i8,
    /// Additional entity-specific data
    pub data: VarInt,
}

impl WriteTo for CAddEntity {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        VarInt(self.entity_id).write(writer)?;
        self.uuid.write(writer)?;
        self.entity_type.write(writer)?;
        writer.write_all(&self.x.to_be_bytes())?;
        writer.write_all(&self.y.to_be_bytes())?;
        writer.write_all(&self.z.to_be_bytes())?;

        // Write velocity as LpVec3 (compressed format)
        // For now, just write zero velocity (single 0 byte) since players don't have physics yet
        // TODO: Implement proper LpVec3 encoding when velocity is needed
        writer.write_all(&[0u8])?;

        self.pitch.write(writer)?;
        self.yaw.write(writer)?;
        self.head_yaw.write(writer)?;
        self.data.write(writer)
    }
}
