use steel_macros::ClientPacket;
use steel_registry::packets::play::C_SYSTEM_CHAT;
use steel_utils::text::TextComponent;

#[derive(ClientPacket, Clone, Debug)]
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

impl steel_utils::serial::WriteTo for CSystemChat {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        let encoded = self.content.encode();
        writer.write_all(&encoded)?;
        self.overlay.write(writer)?;
        Ok(())
    }
}
