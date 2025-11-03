use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::clientbound::config::CLIENTBOUND_DISCONNECT;
use steel_registry::packets::clientbound::play::CLIENTBOUND_DISCONNECT as PLAY_CLIENTBOUND_DISCONNECT;
use steel_utils::text::TextComponent;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(
    CONFIGURATION = "CLIENTBOUND_DISCONNECT",
    PLAY = "PLAY_CLIENTBOUND_DISCONNECT"
)]
pub struct CDisconnectPacket {
    pub reason: TextComponent,
}

impl CDisconnectPacket {
    pub fn new(reason: TextComponent) -> Self {
        Self { reason }
    }
}
