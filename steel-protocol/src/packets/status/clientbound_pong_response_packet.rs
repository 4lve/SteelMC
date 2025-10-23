use steel_macros::PacketWrite;

#[derive(PacketWrite, Clone, Debug)]
pub struct ClientboundPongResponsePacket {
    pub time: i64,
}

impl ClientboundPongResponsePacket {
    pub fn new(time: i64) -> Self {
        Self { time }
    }
}
