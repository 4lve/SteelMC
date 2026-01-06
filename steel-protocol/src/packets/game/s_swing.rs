//! Serverbound swing packet (player arm animation)

use steel_macros::ServerPacket;
use steel_utils::codec::VarInt;
use steel_utils::serial::{ReadFrom, WriteTo};

/// The hand the player is swinging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum InteractionHand {
    MainHand = 0,
    OffHand = 1,
}

impl ReadFrom for InteractionHand {
    fn read(reader: &mut impl std::io::Read) -> std::io::Result<Self> {
        let value = VarInt::read(reader)?;
        match value.0 {
            0 => Ok(Self::MainHand),
            1 => Ok(Self::OffHand),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid interaction hand: {}", value.0),
            )),
        }
    }
}

impl WriteTo for InteractionHand {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        VarInt(*self as i32).write(writer)
    }
}

/// Sent when the player swings their arm
#[derive(ServerPacket, Debug, Clone)]
#[packet_id(Play = S_SWING)]
pub struct SSwing {
    /// The hand being swung
    pub hand: InteractionHand,
}

impl ReadFrom for SSwing {
    fn read(reader: &mut impl std::io::Read) -> std::io::Result<Self> {
        Ok(Self {
            hand: InteractionHand::read(reader)?,
        })
    }
}
