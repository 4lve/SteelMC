use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::login::C_HELLO;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Login = "C_HELLO")]
pub struct CHello {
    #[write_as(as = "string", bound = 20)]
    pub server_id: String,
    #[write_as(as = "vec")]
    pub public_key: Box<[u8]>,
    #[write_as(as = "vec")]
    pub challenge: [u8; 4],
    pub should_authenticate: bool,
}

impl CHello {
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
