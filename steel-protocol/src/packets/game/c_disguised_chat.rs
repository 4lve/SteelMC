use steel_macros::ClientPacket;
use steel_registry::packets::play::C_DISGUISED_CHAT;
use steel_utils::text::TextComponent;

use super::c_player_chat::ChatTypeBound;

/// Clientbound packet for unsigned/disguised chat messages
/// This is sent when the server doesn't have a signed message
#[derive(ClientPacket, Clone, Debug)]
#[packet_id(Play = C_DISGUISED_CHAT)]
pub struct CDisguisedChat {
    pub message: TextComponent,
    pub chat_type: ChatTypeBound,
}

impl CDisguisedChat {
    pub fn new(message: TextComponent, chat_type: ChatTypeBound) -> Self {
        Self { message, chat_type }
    }
}

impl steel_utils::serial::WriteTo for CDisguisedChat {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        // Write message component as NBT
        let encoded = self.message.encode();
        writer.write_all(&encoded)?;

        // Write chat type bound
        self.chat_type.write(writer)?;

        Ok(())
    }
}
