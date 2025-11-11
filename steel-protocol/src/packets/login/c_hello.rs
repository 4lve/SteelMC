use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::login::C_HELLO;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Login = C_HELLO)]
pub struct CHello<'a> {
    #[write(as = "string", bound = 20)]
    pub server_id: String,
    #[write(as = "vec")]
    pub public_key: &'a [u8],
    #[write(as = "vec")]
    pub challenge: [u8; 4],
    pub should_authenticate: bool,
}

impl<'a> CHello<'a> {
    pub fn new(
        server_id: String,
        public_key: &'a [u8],
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
