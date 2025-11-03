use steel_macros::PacketRead;

#[derive(PacketRead, Clone, Debug)]
pub struct SKeyPacket {
    #[read_as(as = "vec")]
    pub key: Vec<u8>,
    #[read_as(as = "vec")]
    pub challenge: Vec<u8>,
}

impl SKeyPacket {
    pub fn new(key: Vec<u8>, challenge: Vec<u8>) -> Self {
        Self { key, challenge }
    }
}
