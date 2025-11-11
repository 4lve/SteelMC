use steel_macros::{ReadFrom, ServerPacket};

#[derive(ReadFrom, ServerPacket, Clone, Debug)]
pub struct SKey {
    #[read(as = "vec")]
    pub key: Vec<u8>,
    #[read(as = "vec")]
    pub challenge: Vec<u8>,
}

impl SKey {
    pub fn new(key: Vec<u8>, challenge: Vec<u8>) -> Self {
        Self { key, challenge }
    }
}
