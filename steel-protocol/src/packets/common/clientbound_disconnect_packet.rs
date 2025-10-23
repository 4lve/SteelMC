use steel_macros::PacketWrite;
use steel_utils::text::TextComponentBase;

#[derive(PacketWrite, Clone, Debug)]
pub struct ClientboundDisconnectPacket {
    pub reason: TextComponentBase,
}

impl ClientboundDisconnectPacket {
    pub fn new(reason: TextComponentBase) -> Self {
        Self { reason }
    }
}
