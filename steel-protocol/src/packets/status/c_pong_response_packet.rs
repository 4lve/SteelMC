use steel_macros::PacketWrite;

#[derive(PacketWrite, Clone, Debug)]
pub struct CPongResponsePacket {
    pub time: i64,
}

impl CPongResponsePacket {
    pub fn new(time: i64) -> Self {
        Self { time }
    }
}
