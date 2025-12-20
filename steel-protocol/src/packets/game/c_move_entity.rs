//! Clientbound move entity packets (for relative position/rotation updates)

use steel_macros::ClientPacket;
use steel_registry::packets::play::{C_MOVE_ENTITY_POS, C_MOVE_ENTITY_POS_ROT, C_MOVE_ENTITY_ROT};
use steel_utils::codec::VarInt;
use steel_utils::serial::WriteTo;

/// Moves an entity by a relative position delta
#[derive(ClientPacket, Debug, Clone)]
#[packet_id(Play = C_MOVE_ENTITY_POS)]
pub struct CMoveEntityPos {
    /// The entity ID
    pub entity_id: i32,
    /// Delta X (in 1/4096 blocks)
    pub delta_x: i16,
    /// Delta Y (in 1/4096 blocks)
    pub delta_y: i16,
    /// Delta Z (in 1/4096 blocks)
    pub delta_z: i16,
    /// Whether the entity is on ground
    pub on_ground: bool,
}

impl WriteTo for CMoveEntityPos {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        VarInt(self.entity_id).write(writer)?;
        self.delta_x.write(writer)?;
        self.delta_y.write(writer)?;
        self.delta_z.write(writer)?;
        self.on_ground.write(writer)
    }
}

/// Moves an entity by a relative position delta and updates rotation
#[derive(ClientPacket, Debug, Clone)]
#[packet_id(Play = C_MOVE_ENTITY_POS_ROT)]
pub struct CMoveEntityPosRot {
    /// The entity ID
    pub entity_id: i32,
    /// Delta X (in 1/4096 blocks)
    pub delta_x: i16,
    /// Delta Y (in 1/4096 blocks)
    pub delta_y: i16,
    /// Delta Z (in 1/4096 blocks)
    pub delta_z: i16,
    /// Yaw (256 = 360 degrees)
    pub yaw: i8,
    /// Pitch (256 = 360 degrees)
    pub pitch: i8,
    /// Whether the entity is on ground
    pub on_ground: bool,
}

impl WriteTo for CMoveEntityPosRot {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        VarInt(self.entity_id).write(writer)?;
        self.delta_x.write(writer)?;
        self.delta_y.write(writer)?;
        self.delta_z.write(writer)?;
        self.yaw.write(writer)?;
        self.pitch.write(writer)?;
        self.on_ground.write(writer)
    }
}

/// Updates only an entity's rotation
#[derive(ClientPacket, Debug, Clone)]
#[packet_id(Play = C_MOVE_ENTITY_ROT)]
pub struct CMoveEntityRot {
    /// The entity ID
    pub entity_id: i32,
    /// Yaw (256 = 360 degrees)
    pub yaw: i8,
    /// Pitch (256 = 360 degrees)
    pub pitch: i8,
    /// Whether the entity is on ground
    pub on_ground: bool,
}

impl WriteTo for CMoveEntityRot {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        VarInt(self.entity_id).write(writer)?;
        self.yaw.write(writer)?;
        self.pitch.write(writer)?;
        self.on_ground.write(writer)
    }
}
