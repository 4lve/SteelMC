use steel_macros::PacketRead;

#[derive(PacketRead, Clone, Debug)]
pub struct ServerboundStatusRequestPacket {}

impl ServerboundStatusRequestPacket {
    pub fn new() -> Self {
        Self {}
    }
}
