use std::{fmt::Debug, io::Cursor};

use steel_utils::{BlockStateId, serial::WriteTo, types::Todo};

use crate::chunk::paletted_container::BlockPalette;

#[derive(Debug)]
pub struct Sections {
    pub sections: Box<[ChunkSection]>,
}

impl Sections {
    pub const fn get(&self) -> &[ChunkSection] {
        &self.sections
    }

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

        let idx = relative_y / BlockPalette::SIZE;
        let relative_y = relative_y % BlockPalette::SIZE;
        println!(
            "setting block at {}, {}, {} to {}",
            relative_x, relative_y, relative_z, value.0
        );
        self.sections[idx]
            .states
            .set(relative_x, relative_y, relative_z, value);
    }
}

#[derive(Debug, Clone)]
pub struct ChunkSection {
    pub states: BlockPalette,
    pub biomes: Todo,
}

impl ChunkSection {
    pub fn new(states: BlockPalette) -> Self {
        Self { states, biomes: () }
    }

    pub fn write(&self, writer: &mut Cursor<Vec<u8>>) {
        self.states.non_empty_block_count().write(writer).unwrap();
    }
}
