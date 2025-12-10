use steel_macros::{ClientPacket, WriteTo};
#[allow(unused_imports)]
use steel_registry::packets::play::C_SYSTEM_CHAT;
use steel_utils::text::TextComponent;

#[derive(ClientPacket, WriteTo)]
#[packet_id(Play = C_SYSTEM_CHAT)]
pub struct CSystemChatMessage {
    pub content: TextComponent,
    pub overlay: bool,
}
