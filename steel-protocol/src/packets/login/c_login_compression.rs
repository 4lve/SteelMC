use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::login::C_LOGIN_COMPRESSION;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Login = "C_LOGIN_COMPRESSION")]
pub struct CLoginCompression {
    #[write_as(as = "var_int")]
    pub threshold: i32,
}

impl CLoginCompression {
    pub fn new(threshold: i32) -> Self {
        Self { threshold }
    }
}
