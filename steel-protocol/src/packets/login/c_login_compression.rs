use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::clientbound::login::CLIENTBOUND_LOGIN_COMPRESSION;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(LOGIN = "CLIENTBOUND_LOGIN_COMPRESSION")]
pub struct CLoginCompressionPacket {
    #[write_as(as = "var_int")]
    pub threshold: i32,
}

impl CLoginCompressionPacket {
    pub fn new(threshold: i32) -> Self {
        Self { threshold }
    }
}
