use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::clientbound::config::CLIENTBOUND_FINISH_CONFIGURATION;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(CONFIGURATION = "CLIENTBOUND_FINISH_CONFIGURATION")]
pub struct CFinishConfigurationPacket {}
