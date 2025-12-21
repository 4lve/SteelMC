use steel_macros::{ClientPacket, WriteTo};

use steel_registry::packets::play::C_FORGET_LEVEL_CHUNK;
use steel_utils::ChunkPos;

#[derive(ClientPacket, WriteTo)]
#[packet_id(Play = C_FORGET_LEVEL_CHUNK)]
pub struct CForgetLevelChunk {
    #[write(as = I64)]
    pub pos: ChunkPos,
}
