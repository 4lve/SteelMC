use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::config::C_CUSTOM_PAYLOAD;
use steel_registry::packets::play::C_CUSTOM_PAYLOAD as PLAY_C_CUSTOM_PAYLOAD;
use steel_utils::Identifier;

#[derive(WriteTo, ClientPacket, Clone, Debug)]
#[packet_id(Config = C_CUSTOM_PAYLOAD, Play = PLAY_C_CUSTOM_PAYLOAD)]
pub struct CCustomPayload<'a> {
    pub identifier: Identifier,
    #[write(as = "vec")]
    pub payload: &'a [u8],
}

impl<'a> CCustomPayload<'a> {
    pub fn new(identifier: Identifier, payload: &'a [u8]) -> Self {
        Self {
            identifier,
            payload,
        }
    }
}
