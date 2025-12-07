use std::io::{Result, Write};

use steel_macros::ClientPacket;
use steel_registry::packets::play::C_LIGHT_UPDATE;
use steel_utils::{ChunkPos, codec::VarInt, serial::WriteTo};

use super::c_level_chunk_with_light::LightUpdatePacketData;

#[derive(ClientPacket, Debug, Clone)]
#[packet_id(Play = C_LIGHT_UPDATE)]
pub struct CLightUpdate {
    pub pos: ChunkPos,
    pub light_data: LightUpdatePacketData,
}

// Custom WriteTo implementation because CLightUpdate writes chunk pos as VarInt,
// unlike CLevelChunkWithLight which writes as i32
impl WriteTo for CLightUpdate {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        // Write chunk position as VarInts (not i32)
        VarInt(self.pos.0.x).write(writer)?;
        VarInt(self.pos.0.y).write(writer)?;
        // Write light data
        self.light_data.write(writer)?;
        Ok(())
    }
}
