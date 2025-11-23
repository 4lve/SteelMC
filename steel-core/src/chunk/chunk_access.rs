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
    /// Gets the index of the status.
    #[must_use]
    pub fn get_index(self) -> usize {
        self as usize
    }

    /// Gets the status from an index.
    #[must_use]
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Empty),
            1 => Some(Self::StructureStarts),
            2 => Some(Self::StructureReferences),
            3 => Some(Self::Biomes),
            4 => Some(Self::Noise),
            5 => Some(Self::Surface),
            6 => Some(Self::Carvers),
            7 => Some(Self::Features),
            8 => Some(Self::InitializeLight),
            9 => Some(Self::Light),
            10 => Some(Self::Spawn),
            11 => Some(Self::Full),
            _ => None,
        }
    }
}

impl ChunkStatus {
    /// Gets the next status in the generation order.
    /// # Panics
    /// This function will panic if the chunk is at the Full status.
    #[must_use]
    pub fn next(self) -> Option<Self> {
        match self {
            Self::Empty => Some(Self::StructureStarts),
            Self::StructureStarts => Some(Self::StructureReferences),
            Self::StructureReferences => Some(Self::Biomes),
            Self::Biomes => Some(Self::Noise),
            Self::Noise => Some(Self::Surface),
            Self::Surface => Some(Self::Carvers),
            Self::Carvers => Some(Self::Features),
            Self::Features => Some(Self::InitializeLight),
            Self::InitializeLight => Some(Self::Light),
            Self::Light => Some(Self::Spawn),
            Self::Spawn => Some(Self::Full),
            Self::Full => None,
        }
    }

    /// Gets the parent status in the generation order.
    #[must_use]
    pub fn parent(self) -> Option<Self> {
        match self {
            Self::Empty => None,
            Self::StructureStarts => Some(Self::Empty),
            Self::StructureReferences => Some(Self::StructureStarts),
            Self::Biomes => Some(Self::StructureReferences),
            Self::Noise => Some(Self::Biomes),
            Self::Surface => Some(Self::Noise),
            Self::Carvers => Some(Self::Surface),
            Self::Features => Some(Self::Carvers),
            Self::InitializeLight => Some(Self::Features),
            Self::Light => Some(Self::InitializeLight),
            Self::Spawn => Some(Self::Light),
            Self::Full => Some(Self::Spawn),
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
