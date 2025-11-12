//! This module contains the `LevelChunk` struct, which is a chunk that is ready to be sent to the client.
use steel_utils::ChunkPos;

use crate::chunk::{proto_chunk::ProtoChunk, section::Sections};

/// A chunk that is ready to be sent to the client.
#[derive(Debug)]
pub struct LevelChunk {
    /// The sections of the chunk.
    pub sections: Sections,
    /// The position of the chunk.
    pub pos: ChunkPos,
}

impl LevelChunk {
    /// Creates a new `LevelChunk` from a `ProtoChunk`.
    #[must_use]
    pub fn from_proto(proto_chunk: ProtoChunk) -> Self {
        Self {
            sections: proto_chunk.sections,
            pos: proto_chunk.pos,
        }
    }
}
