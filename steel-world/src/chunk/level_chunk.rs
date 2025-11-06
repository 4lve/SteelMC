use steel_utils::BlockStateId;

use crate::chunk::{paletted_container::BlockPalette, section::ChunkSection};

#[derive(Debug)]
pub struct LevelChunk {
    pub sections: Box<[ChunkSection]>,
}

impl LevelChunk {
    pub fn get_relative_block(
        &self,
        relative_x: usize,
        relative_y: usize,
        relative_z: usize,
    ) -> Option<BlockStateId> {
        debug_assert!(relative_x < BlockPalette::SIZE);
        debug_assert!(relative_z < BlockPalette::SIZE);

        let section_index = relative_y / BlockPalette::SIZE;
        let relative_y = relative_y % BlockPalette::SIZE;
        self.sections
            .get(section_index)
            .map(|section| section.states.get(relative_x, relative_y, relative_z))
    }

    pub fn set_relative_block(
        &mut self,
        relative_x: usize,
        relative_y: usize,
        relative_z: usize,
        value: BlockStateId,
    ) {
        debug_assert!(relative_x < BlockPalette::SIZE);
        debug_assert!(relative_z < BlockPalette::SIZE);

        let section_index = relative_y / BlockPalette::SIZE;
        let relative_y = relative_y % BlockPalette::SIZE;
        println!(
            "setting block at {}, {}, {} to {}",
            relative_x, relative_y, relative_z, value.0
        );
        self.sections[section_index]
            .states
            .set(relative_x, relative_y, relative_z, value);
    }
}
