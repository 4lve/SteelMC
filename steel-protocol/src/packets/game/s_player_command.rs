//! Serverbound player command packet (sprinting, sneaking, etc.)

use steel_macros::ServerPacket;
use steel_utils::codec::VarInt;
use steel_utils::serial::{ReadFrom, WriteTo};

/// Player command actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum PlayerCommandAction {
    StartSneaking = 0,
    StopSneaking = 1,
    LeaveBed = 2,
    StartSprinting = 3,
    StopSprinting = 4,
    StartJumpWithHorse = 5,
    StopJumpWithHorse = 6,
    OpenHorseInventory = 7,
    StartFlyingWithElytra = 8,
}

impl ReadFrom for PlayerCommandAction {
    fn read(reader: &mut impl std::io::Read) -> std::io::Result<Self> {
        let value = VarInt::read(reader)?;
        match value.0 {
            0 => Ok(Self::StartSneaking),
            1 => Ok(Self::StopSneaking),
            2 => Ok(Self::LeaveBed),
            3 => Ok(Self::StartSprinting),
            4 => Ok(Self::StopSprinting),
            5 => Ok(Self::StartJumpWithHorse),
            6 => Ok(Self::StopJumpWithHorse),
            7 => Ok(Self::OpenHorseInventory),
            8 => Ok(Self::StartFlyingWithElytra),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid player command action: {}", value.0),
            )),
        }
    }
}

impl WriteTo for PlayerCommandAction {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        VarInt(*self as i32).write(writer)
    }
}

/// Sent when the player performs a command (sprint, sneak, etc.)
#[derive(ServerPacket, Debug, Clone)]
#[packet_id(Play = S_PLAYER_COMMAND)]
pub struct SPlayerCommand {
    /// The entity ID (should be player's own ID)
    pub entity_id: VarInt,
    /// The action being performed
    pub action: PlayerCommandAction,
    /// Additional data (for horse jumps)
    pub data: VarInt,
}

impl ReadFrom for SPlayerCommand {
    fn read(reader: &mut impl std::io::Read) -> std::io::Result<Self> {
        Ok(Self {
            entity_id: VarInt::read(reader)?,
            action: PlayerCommandAction::read(reader)?,
            data: VarInt::read(reader)?,
        })
    }
}
