use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::play::C_SET_HEALTH;

/// Sent by the server to update the client's health, food, and saturation.
#[derive(WriteTo, ClientPacket, Clone, Debug)]
#[packet_id(Play = C_SET_HEALTH)]
pub struct CSetHealth {
    pub health: f32,
    #[write(as = VarInt)]
    pub food: i32,
    pub food_saturation: f32,
}