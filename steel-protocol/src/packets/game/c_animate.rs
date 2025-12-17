//! Clientbound animate packet (broadcasts animations like arm swing)

use steel_macros::ClientPacket;
use steel_registry::packets::play::C_ANIMATE;
use steel_utils::codec::VarInt;
use steel_utils::serial::WriteTo;

/// Animation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AnimationType {
    SwingMainArm = 0,
    Hurt = 1,
    LeaveBed = 2,
    SwingOffhand = 3,
    CriticalHit = 4,
    MagicCriticalHit = 5,
}

/// Broadcasts an entity animation to nearby players
#[derive(ClientPacket, Debug, Clone)]
#[packet_id(Play = C_ANIMATE)]
pub struct CAnimate {
    /// The entity performing the animation
    pub entity_id: VarInt,
    /// The animation type
    pub animation: u8,
}

impl CAnimate {
    /// Creates a new animate packet
    pub fn new(entity_id: i32, animation: AnimationType) -> Self {
        Self {
            entity_id: VarInt(entity_id),
            animation: animation as u8,
        }
    }

    /// Creates a main hand swing animation
    pub fn swing_main_hand(entity_id: i32) -> Self {
        Self::new(entity_id, AnimationType::SwingMainArm)
    }

    /// Creates an offhand swing animation
    pub fn swing_offhand(entity_id: i32) -> Self {
        Self::new(entity_id, AnimationType::SwingOffhand)
    }
}

impl WriteTo for CAnimate {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.entity_id.write(writer)?;
        self.animation.write(writer)
    }
}
