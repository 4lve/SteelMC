//! Packet sent to spawn an entity (including players) for the client.

use steel_macros::ClientPacket;
use steel_registry::packets::play::C_ADD_ENTITY;
use steel_utils::codec::VarInt;
use steel_utils::serial::WriteTo;
use uuid::Uuid;

/// Spawns an entity on the client.
#[derive(ClientPacket, Clone, Debug)]
#[packet_id(Play = C_ADD_ENTITY)]
pub struct CAddEntity {
    /// The entity ID (used for all future references to this entity)
    pub id: i32,
    /// The entity's UUID
    pub uuid: Uuid,
    /// The entity type (from registry)
    pub entity_type: i32,
    /// X position
    pub x: f64,
    /// Y position
    pub y: f64,
    /// Z position
    pub z: f64,
    /// Pitch (vertical rotation) as angle byte
    pub x_rot: i8,
    /// Yaw (horizontal rotation) as angle byte
    pub y_rot: i8,
    /// Head yaw as angle byte
    pub head_y_rot: i8,
    /// Entity data value (varies by entity type)
    pub data: i32,
}

impl WriteTo for CAddEntity {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        VarInt(self.id).write(writer)?;
        self.uuid.write(writer)?;
        VarInt(self.entity_type).write(writer)?;
        writer.write_all(&self.x.to_be_bytes())?;
        writer.write_all(&self.y.to_be_bytes())?;
        writer.write_all(&self.z.to_be_bytes())?;
        // Velocity as LpVec3 (zero velocity = single 0 byte)
        writer.write_all(&[0u8])?;
        self.x_rot.write(writer)?;
        self.y_rot.write(writer)?;
        self.head_y_rot.write(writer)?;
        VarInt(self.data).write(writer)
    }
}

impl CAddEntity {
    /// Creates a new CAddEntity packet for spawning a player.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn player(
        id: i32,
        uuid: Uuid,
        entity_type_id: i32,
        x: f64,
        y: f64,
        z: f64,
        yaw: f32,
        pitch: f32,
    ) -> Self {
        Self {
            id,
            uuid,
            entity_type: entity_type_id,
            x,
            y,
            z,
            x_rot: ((pitch / 360.0) * 256.0) as i8,
            y_rot: ((yaw / 360.0) * 256.0) as i8,
            head_y_rot: ((yaw / 360.0) * 256.0) as i8,
            data: 0,
        }
    }
}
