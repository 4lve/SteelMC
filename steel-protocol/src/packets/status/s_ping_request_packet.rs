use steel_macros::PacketRead;

#[derive(PacketRead, Clone, Debug)]
pub struct SPingRequestPacket {
    pub time: i64,
}

impl SPingRequestPacket {
    pub fn new(time: i64) -> Self {
        Self { time }
    }
}
