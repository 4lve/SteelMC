use steel_macros::PacketWrite;
use steel_utils::text::TextComponentBase;

#[derive(PacketWrite, Clone, Debug)]
pub struct CLoginDisconnectPacket {
    #[write_as(as = "json")]
    pub reason: TextComponentBase,
}

impl CLoginDisconnectPacket {
    pub fn new(reason: TextComponentBase) -> Self {
        Self { reason }
    }
}
