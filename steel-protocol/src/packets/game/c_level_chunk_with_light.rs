use std::io::{Result, Write};

use steel_macros::ClientPacket;

use steel_registry::packets::play::C_LEVEL_CHUNK_WITH_LIGHT;
use steel_utils::{ChunkPos, codec::VarInt, serial::WriteTo};

#[derive(ClientPacket)]
#[packet_id(Play = C_LEVEL_CHUNK_WITH_LIGHT)]
pub struct CLevelChunkWithLight {
    pub pos: ChunkPos,
    pub heightmaps: (),
    pub chunk: (),
}

impl WriteTo for CLevelChunkWithLight {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.pos.write(writer)?;

        //Heightmaps
        VarInt(0).write(writer)?;
        Ok(())
    }
}
