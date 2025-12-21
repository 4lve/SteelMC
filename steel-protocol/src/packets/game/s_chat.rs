use steel_macros::{ReadFrom, ServerPacket};

#[derive(ReadFrom, ServerPacket, Clone, Debug)]
pub struct SChat {
    #[read(as = Prefixed(VarInt), bound = 256)]
    pub message: String,

    pub timestamp: i64,

    pub salt: i64,

    pub signature: Option<[u8; 256]>,

    #[read(as = VarInt)]
    pub offset: i32,

    pub acknowledged: [u8; 3],

    pub checksum: u8,
}
