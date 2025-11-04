use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::config::C_CUSTOM_PAYLOAD;
use steel_registry::packets::play::C_CUSTOM_PAYLOAD as PLAY_C_CUSTOM_PAYLOAD;
use steel_utils::ResourceLocation;

#[derive(WriteTo, ClientPacket, Clone, Debug)]
#[packet_id(Config = "C_CUSTOM_PAYLOAD", Play = "PLAY_C_CUSTOM_PAYLOAD")]
pub struct CCustomPayload {
    pub resource_location: ResourceLocation,
    #[write_as(as = "vec")]
    pub payload: Box<[u8]>,
}

impl CCustomPayload {
    pub fn new(resource_location: ResourceLocation, payload: Box<[u8]>) -> Self {
        Self {
            resource_location,
            payload,
        }
    }
}
