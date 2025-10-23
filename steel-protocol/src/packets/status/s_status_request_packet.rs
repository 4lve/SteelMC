use steel_macros::PacketRead;

#[derive(PacketRead, Clone, Debug)]
pub struct SStatusRequestPacket {}

impl SStatusRequestPacket {
    pub fn new() -> Self {
        Self {}
    }
}
