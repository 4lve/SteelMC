use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::play::C_SYSTEM_CHAT;
use steel_utils::text::TextComponent;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_SYSTEM_CHAT)]
pub struct CSystemChat {
    pub content: TextComponent,
    pub overlay: bool,
}

impl CSystemChat {
    pub fn new(content: TextComponent, overlay: bool) -> Self {
        Self { content, overlay }
    }
}
