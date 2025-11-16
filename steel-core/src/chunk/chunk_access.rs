//! This module contains the `ChunkAccess` enum, which is used to access chunks in different states.
use steel_utils::BlockStateId;

use crate::chunk::{level_chunk::LevelChunk, proto_chunk::ProtoChunk};

/// The status of a chunk.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum ChunkStatus {
    /// The chunk is empty.
    Empty,
    /// The chunk is being processed for structure starts.
    StructureStarts,
    /// The chunk is being processed for structure references.
    StructureReferences,
    /// The chunk is being processed for biomes.
    Biomes,
    /// The chunk is being processed for noise.
    Noise,
    /// The chunk is being processed for surfaces.
    Surface,
    /// The chunk is being processed for carvers.
    Carvers,
    /// The chunk is being processed for features.
    Features,
    /// The chunk is being initialized for light.
    InitializeLight,
    /// The chunk is being lit.
    Light,
    /// The chunk is being spawned.
    Spawn,
    /// The chunk is fully generated.
    Full,
}

impl ChunkStatus {
    /// Gets the next status in the generation order.
    /// # Panics
    /// This function will panic if the chunk is at the Full status.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Empty => Self::StructureStarts,
            Self::StructureStarts => Self::StructureReferences,
            Self::StructureReferences => Self::Biomes,
            Self::Biomes => Self::Noise,
            Self::Noise => Self::Surface,
            Self::Surface => Self::Carvers,
            Self::Carvers => Self::Features,
            Self::Features => Self::InitializeLight,
            Self::InitializeLight => Self::Light,
            Self::Light => Self::Spawn,
            Self::Spawn => Self::Full,
            Self::Full => unreachable!(),
        }
    }
}

/// An enum that allows access to a chunk in different states.
pub enum ChunkAccess {
    /// A fully generated chunk.
    Full(LevelChunk),
    /// A chunk that is still being generated.
    Proto(ProtoChunk),
}

impl ChunkAccess {
    /// Converts a proto chunk into a full chunk.
    ///
    /// # Panics
    /// This function will panic if the chunk is already a full chunk.
    #[must_use]
    pub fn into_full(self) -> Self {
        match self {
            Self::Proto(proto_chunk) => Self::Full(LevelChunk::from_proto(proto_chunk)),
            Self::Full(_) => unreachable!(),
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
        let sections = match self {
            Self::Full(chunk) => &chunk.sections,
            Self::Proto(proto_chunk) => &proto_chunk.sections,
        };

        sections.get_relative_block(relative_x, relative_y, relative_z)
    }

    /// Sets a block at a relative position in the chunk.
    pub fn set_relative_block(
        &mut self,
        relative_x: usize,
        relative_y: usize,
        relative_z: usize,
        value: BlockStateId,
    ) {
        let sections = match self {
            Self::Full(chunk) => &mut chunk.sections,
            Self::Proto(proto_chunk) => &mut proto_chunk.sections,
        };

        sections.set_relative_block(relative_x, relative_y, relative_z, value);
    }
}
