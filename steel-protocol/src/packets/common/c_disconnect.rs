use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::config::C_DISCONNECT;
use steel_registry::packets::play::C_DISCONNECT as PLAY_C_DISCONNECT;
use steel_utils::text::TextComponent;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Config = "C_DISCONNECT", Play = "PLAY_C_DISCONNECT")]
pub struct CDisconnect {
    pub reason: TextComponent,
}

impl CDisconnect {
    pub fn new(reason: TextComponent) -> Self {
        Self { reason }
    }
}
