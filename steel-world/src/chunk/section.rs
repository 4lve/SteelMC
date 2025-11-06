use std::fmt::Debug;

use steel_utils::types::Todo;

use crate::chunk::paletted_container::BlockPalette;

#[derive(Debug, Clone)]
pub struct SubChunk {}

#[derive(Debug, Clone)]
pub struct ChunkSection {
    pub states: BlockPalette,
    pub biomes: Todo,
}

impl ChunkSection {
    pub fn new(states: BlockPalette) -> Self {
        Self { states, biomes: () }
    }

    pub fn write(&self, _buf: &mut Vec<u8>) {
        todo!()
    }
}
