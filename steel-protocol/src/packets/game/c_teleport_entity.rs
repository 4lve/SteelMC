//! Clientbound teleport entity packet (sets absolute entity position)

use steel_macros::ClientPacket;
use steel_registry::packets::play::C_TELEPORT_ENTITY;
use steel_utils::serial::WriteTo;

/// Teleports an entity to an absolute position
#[derive(ClientPacket, Debug, Clone)]
#[packet_id(Play = C_TELEPORT_ENTITY)]
pub struct CTeleportEntity {
    /// The entity's ID
    pub entity_id: i32,
    /// Position X
    pub x: f64,
    /// Position Y
    pub y: f64,
    /// Position Z
    pub z: f64,
    /// Delta movement X
    pub delta_x: f64,
    /// Delta movement Y
    pub delta_y: f64,
    /// Delta movement Z
    pub delta_z: f64,
    /// Yaw (rotation around Y axis) in degrees
    pub yaw: f32,
    /// Pitch (rotation around X axis) in degrees
    pub pitch: f32,
    /// Relative flags (bitfield)
    pub relatives: i32,
    /// Whether the entity is on the ground
    pub on_ground: bool,
}

impl WriteTo for CTeleportEntity {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        // VarInt entity ID
        steel_utils::codec::VarInt(self.entity_id).write(writer)?;

        // Position Vec3 (3 doubles)
        writer.write_all(&self.x.to_be_bytes())?;
        writer.write_all(&self.y.to_be_bytes())?;
        writer.write_all(&self.z.to_be_bytes())?;

        // Delta movement Vec3 (3 doubles)
        writer.write_all(&self.delta_x.to_be_bytes())?;
        writer.write_all(&self.delta_y.to_be_bytes())?;
        writer.write_all(&self.delta_z.to_be_bytes())?;

        // Rotation (2 floats)
        writer.write_all(&self.yaw.to_be_bytes())?;
        writer.write_all(&self.pitch.to_be_bytes())?;

        // Relative flags (int, not VarInt!)
        writer.write_all(&self.relatives.to_be_bytes())?;

        // On ground (boolean)
        self.on_ground.write(writer)?;

        Ok(())
    }
}
