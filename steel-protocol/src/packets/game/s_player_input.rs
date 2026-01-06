//! Serverbound player input packet (movement keys, jump, sneak)

use steel_macros::ServerPacket;
use steel_utils::serial::ReadFrom;

/// Player input flags
#[derive(Debug, Clone, Copy)]
pub struct PlayerInputFlags {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sneak: bool,
    pub sprint: bool,
}

impl PlayerInputFlags {
    const FORWARD: u8 = 0x01;
    const BACKWARD: u8 = 0x02;
    const LEFT: u8 = 0x04;
    const RIGHT: u8 = 0x08;
    const JUMP: u8 = 0x10;
    const SNEAK: u8 = 0x20;
    const SPRINT: u8 = 0x40;

    /// Creates flags from a byte
    pub fn from_byte(byte: u8) -> Self {
        Self {
            forward: (byte & Self::FORWARD) != 0,
            backward: (byte & Self::BACKWARD) != 0,
            left: (byte & Self::LEFT) != 0,
            right: (byte & Self::RIGHT) != 0,
            jump: (byte & Self::JUMP) != 0,
            sneak: (byte & Self::SNEAK) != 0,
            sprint: (byte & Self::SPRINT) != 0,
        }
    }

    /// Converts flags to a byte
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0u8;
        if self.forward {
            byte |= Self::FORWARD;
        }
        if self.backward {
            byte |= Self::BACKWARD;
        }
        if self.left {
            byte |= Self::LEFT;
        }
        if self.right {
            byte |= Self::RIGHT;
        }
        if self.jump {
            byte |= Self::JUMP;
        }
        if self.sneak {
            byte |= Self::SNEAK;
        }
        if self.sprint {
            byte |= Self::SPRINT;
        }
        byte
    }
}

/// Sent when the player presses or releases movement/action keys
#[derive(ServerPacket, Debug, Clone)]
#[packet_id(Play = S_PLAYER_INPUT)]
pub struct SPlayerInput {
    /// Input flags (bit mask)
    pub flags: PlayerInputFlags,
}

impl ReadFrom for SPlayerInput {
    fn read(reader: &mut impl std::io::Read) -> std::io::Result<Self> {
        let byte = u8::read(reader)?;
        Ok(Self {
            flags: PlayerInputFlags::from_byte(byte),
        })
    }
}
