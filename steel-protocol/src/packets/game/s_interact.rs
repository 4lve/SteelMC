//! Serverbound interact packet (attacking, interacting with entities)

use steel_macros::ServerPacket;
use steel_utils::codec::VarInt;
use steel_utils::serial::ReadFrom;

use super::s_swing::InteractionHand;

/// Type of interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum InteractionType {
    Interact = 0,
    Attack = 1,
    InteractAt = 2,
}

/// Sent when the player interacts with an entity
#[derive(ServerPacket, Debug, Clone)]
#[packet_id(Play = S_INTERACT)]
pub struct SInteract {
    /// The entity ID being interacted with
    pub entity_id: VarInt,
    /// The type of interaction
    pub interaction_type: InteractionType,
    /// Target position for InteractAt (x, y, z)
    pub target_pos: Option<(f32, f32, f32)>,
    /// Hand used for interaction
    pub hand: Option<InteractionHand>,
    /// Whether the player is sneaking
    pub sneaking: bool,
}

impl ReadFrom for SInteract {
    fn read(reader: &mut impl std::io::Read) -> std::io::Result<Self> {
        let entity_id = VarInt::read(reader)?;
        let type_id = VarInt::read(reader)?;

        let interaction_type = match type_id.0 {
            0 => InteractionType::Interact,
            1 => InteractionType::Attack,
            2 => InteractionType::InteractAt,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid interaction type: {}", type_id.0),
                ));
            }
        };

        let (target_pos, hand) = match interaction_type {
            InteractionType::Interact => {
                let hand = InteractionHand::read(reader)?;
                (None, Some(hand))
            }
            InteractionType::Attack => (None, None),
            InteractionType::InteractAt => {
                let x = f32::read(reader)?;
                let y = f32::read(reader)?;
                let z = f32::read(reader)?;
                let hand = InteractionHand::read(reader)?;
                (Some((x, y, z)), Some(hand))
            }
        };

        let sneaking = bool::read(reader)?;

        Ok(Self {
            entity_id,
            interaction_type,
            target_pos,
            hand,
            sneaking,
        })
    }
}
