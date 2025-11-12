//! A proto chunk is a chunk that is still being generated.
use steel_utils::ChunkPos;

use crate::chunk::section::Sections;

/// A chunk that is still being generated.
#[derive(Debug)]
pub struct ProtoChunk {
    /// The sections of the chunk.
    pub sections: Sections,
    /// The position of the chunk.
    pub pos: ChunkPos,
}
