use steel_macros::PacketWrite;
use steel_utils::text::TextComponentBase;

#[derive(PacketWrite, Clone, Debug)]
pub struct CDisconnectPacket {
    pub reason: TextComponentBase,
}

impl CDisconnectPacket {
    pub fn new(reason: TextComponentBase) -> Self {
        Self { reason }
    }
}
