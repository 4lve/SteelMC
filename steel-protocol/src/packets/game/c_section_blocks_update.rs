use std::io::{Result, Write};

use steel_macros::ClientPacket;
use steel_registry::packets::play::C_SECTION_BLOCKS_UPDATE;
use steel_utils::{
    BlockPos, BlockStateId, SectionPos,
    codec::{VarInt, VarLong},
    serial::WriteTo,
};

#[derive(ClientPacket, Clone, Debug)]
#[packet_id(Play = C_SECTION_BLOCKS_UPDATE)]
pub struct CSectionBlocksUpdate {
    pub section_pos: SectionPos,
    pub changes: Vec<BlockChange>,
}

#[derive(Clone, Debug)]
pub struct BlockChange {
    pub pos: BlockPos,
    pub block_state: BlockStateId,
}

impl WriteTo for CSectionBlocksUpdate {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        // Write section position
        self.section_pos.write(writer)?;

        // Write number of changes
        VarInt(self.changes.len() as i32).write(writer)?;

        // Write each change as a packed VarLong
        for change in &self.changes {
            // Calculate section-relative position (12 bits total: 4 bits each for x, y, z)
            let section_x = change.pos.0.x & 0xF;
            let section_y = change.pos.0.y & 0xF;
            let section_z = change.pos.0.z & 0xF;

            // Pack as: (section_x << 8) | (section_z << 4) | section_y
            let packed_pos = ((section_x << 8) | (section_z << 4) | section_y) as u16;

            // Pack as: (block_state_id << 12) | packed_pos
            let block_id = i32::from(change.block_state.0);
            let packed = i64::from(block_id) << 12 | i64::from(packed_pos);

            VarLong(packed).write(writer)?;
        }

        Ok(())
    }
}
