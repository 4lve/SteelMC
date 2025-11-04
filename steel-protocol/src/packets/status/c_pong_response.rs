use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::status::C_PONG_RESPONSE;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Status = "C_PONG_RESPONSE")]
pub struct CPongResponse {
    pub time: i64,
}

impl CPongResponse {
    pub fn new(time: i64) -> Self {
        Self { time }
    }
}
