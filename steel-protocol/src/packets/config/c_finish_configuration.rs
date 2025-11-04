use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::config::C_FINISH_CONFIGURATION;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(CONFIG = "C_FINISH_CONFIGURATION")]
pub struct CFinishConfiguration {}
