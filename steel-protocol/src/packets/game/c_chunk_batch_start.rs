use steel_macros::{ClientPacket, WriteTo};

use steel_registry::packets::play::C_CHUNK_BATCH_START;

#[derive(ClientPacket, WriteTo)]
#[packet_id(Play = C_CHUNK_BATCH_START)]
pub struct CChunkBatchStart {}
