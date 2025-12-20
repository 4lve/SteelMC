//! This module contains the `Sections` and `ChunkSection` structs.
use std::{fmt::Debug, io::Cursor, sync::Arc};

use steel_utils::{BlockStateId, locks::SyncRwLock, serial::WriteTo};

use crate::chunk::{
    light_storage::LightStorage,
    paletted_container::{BiomePalette, BlockPalette},
    sky_light_sources::ChunkSkyLightSources,
};

/// A collection of chunk sections.
#[derive(Debug, Clone)]
pub struct Sections {
    /// The sections in the collection.
    pub sections: Box<[Arc<SyncRwLock<ChunkSection>>]>,
    /// Sky light data for each section.
    /// Note: Length is `sections.len() + 2` for padding above and below.
    pub sky_light: Box<[Arc<SyncRwLock<LightStorage>>]>,
    /// Block light data for each section.
    /// Note: Length is `sections.len() + 2` for padding above and below.
    pub block_light: Box<[Arc<SyncRwLock<LightStorage>>]>,
    /// Sky light source tracking for optimization.
    /// Wrapped in `SyncRwLock` for interior mutability.
    pub sky_light_sources: Arc<SyncRwLock<ChunkSkyLightSources>>,
}

impl Sections {
    /// Creates a new `Sections` from a box of owned `ChunkSection`s.
    #[must_use]
    pub fn from_owned(sections: Box<[ChunkSection]>) -> Self {
        let section_count = sections.len();
        let sky_light = (0..(section_count + 2))
            .map(|_| Arc::new(SyncRwLock::new(LightStorage::new_empty())))
            .collect();
        let block_light = (0..(section_count + 2))
            .map(|_| Arc::new(SyncRwLock::new(LightStorage::new_empty())))
            .collect();

        Self {
            sections: sections
                .into_iter()
                .map(|section| Arc::new(SyncRwLock::new(section)))
                .collect(),
            sky_light,
            block_light,
            sky_light_sources: Arc::new(SyncRwLock::new(ChunkSkyLightSources::default())),
        }
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
        self.sections.get(section_index).map(|section| {
            section
                .read()
                .states
                .get(relative_x, relative_y, relative_z)
        })
    }

    /// Sets a block at a relative position in the chunk.
    pub fn set_relative_block(
        &self,
        relative_x: usize,
        relative_y: usize,
        relative_z: usize,
        value: BlockStateId,
    ) {
        debug_assert!(relative_x < BlockPalette::SIZE);
        debug_assert!(relative_z < BlockPalette::SIZE);

        let idx = relative_y / BlockPalette::SIZE;
        let relative_y = relative_y % BlockPalette::SIZE;
        //println!(
        //    "setting block at {}, {}, {} to {}",
        //    relative_x, relative_y, relative_z, value.0
        //);
        self.sections[idx]
            .write()
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
    pub biomes: BiomePalette,
}

impl ChunkSection {
    /// Creates a new chunk section.
    #[must_use]
    pub fn new(states: BlockPalette) -> Self {
        Self {
            states,
            biomes: BiomePalette::Homogeneous(0),
        }
    }

    /// Creates a new empty chunk section.
    #[must_use]
    pub fn new_empty() -> Self {
        Self {
            states: BlockPalette::Homogeneous(BlockStateId(0)),
            biomes: BiomePalette::Homogeneous(0),
        }
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

        self.states
            .write(writer)
            .expect("Failed to write block states");
        self.biomes.write(writer).expect("Failed to write biomes");
    }
}
