use steel_macros::{ClientPacket, WriteTo};

use steel_registry::packets::play::C_CHUNK_BATCH_FINISHED;

#[derive(ClientPacket, WriteTo)]
#[packet_id(Play = C_CHUNK_BATCH_FINISHED)]
pub struct CChunkBatchFinished {
    #[write(as = "var_int")]
    pub batch_size: i32,
}
