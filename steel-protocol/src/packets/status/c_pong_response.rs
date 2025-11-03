use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::clientbound::status::CLIENTBOUND_PONG_RESPONSE;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(STATUS = "CLIENTBOUND_PONG_RESPONSE")]
pub struct CPongResponsePacket {
    pub time: i64,
}

impl CPongResponsePacket {
    pub fn new(time: i64) -> Self {
        Self { time }
    }
}
