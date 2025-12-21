use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::play::C_SET_CHUNK_CACHE_CENTER;

#[derive(ClientPacket, WriteTo)]
#[packet_id(Play = C_SET_CHUNK_CACHE_CENTER)]
pub struct CSetChunkCenter {
    #[write(as = VarInt)]
    pub x: i32,
    #[write(as = VarInt)]
    pub y: i32,
}
