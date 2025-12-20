//! Clientbound player position packet (teleports the player to a position)

use steel_macros::ClientPacket;
use steel_registry::packets::play::C_PLAYER_POSITION;
use steel_utils::codec::VarInt;
use steel_utils::serial::WriteTo;

/// Teleports the player to a position
#[derive(ClientPacket, Debug, Clone)]
#[packet_id(Play = C_PLAYER_POSITION)]
pub struct CPlayerPosition {
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
    /// Yaw in degrees
    pub yaw: f32,
    /// Pitch in degrees
    pub pitch: f32,
    /// Relative flags (bitfield)
    pub relatives: i32,
    /// Teleport ID for confirmation
    pub teleport_id: i32,
}

impl WriteTo for CPlayerPosition {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        // Teleport ID (VarInt) - FIRST!
        VarInt(self.teleport_id).write(writer)?;

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

        Ok(())
    }
}
