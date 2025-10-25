use steel_macros::PacketWrite;

#[derive(PacketWrite, Clone, Debug)]
pub struct CHelloPacket {
    #[write_as(as = "string", bound = 20)]
    pub server_id: String,
    #[write_as(as = "vec")]
    pub public_key: Box<[u8]>,
    #[write_as(as = "vec")]
    pub challenge: [u8; 4],
    pub should_authenticate: bool,
}

impl CHelloPacket {
    pub fn new(
        server_id: String,
        public_key: Box<[u8]>,
        challenge: [u8; 4],
        should_authenticate: bool,
    ) -> Self {
        Self {
            server_id,
            public_key,
            challenge,
            should_authenticate,
        }
    }
}
