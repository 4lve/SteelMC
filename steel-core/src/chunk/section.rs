//! This module contains the `Sections` and `ChunkSection` structs.
use std::{fmt::Debug, io::Cursor};

use steel_utils::{BlockStateId, serial::WriteTo, types::Todo};

use crate::chunk::paletted_container::BlockPalette;

/// A collection of chunk sections.
#[derive(Debug)]
pub struct Sections {
    /// The sections in the collection.
    pub sections: Box<[ChunkSection]>,
}

impl Sections {
    /// Gets the sections in the collection.
    #[must_use]
    pub const fn get(&self) -> &[ChunkSection] {
        &self.sections
    }

    /// Gets a block at a relative position in the chunk.
    #[must_use]
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

    /// Sets a block at a relative position in the chunk.
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

/// A chunk section.
#[derive(Debug, Clone)]
pub struct ChunkSection {
    /// The block states in the section.
    pub states: BlockPalette,
    /// The biomes in the section.
    pub biomes: Todo,
}

impl ChunkSection {
    /// Creates a new chunk section.
    #[must_use]
    pub fn new(states: BlockPalette) -> Self {
        Self { states, biomes: () }
    }

    /// Writes the chunk section to a writer.
    ///
    /// # Panics
    /// - If the writer fails to write.
    pub fn write(&self, writer: &mut Cursor<Vec<u8>>) {
        self.states
            .non_empty_block_count()
            .write(writer)
            .expect("Failed to write block count");
    }
}
