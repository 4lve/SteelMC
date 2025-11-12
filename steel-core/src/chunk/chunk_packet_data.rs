//! This module contains the `ChunkPacketData` struct, which is used to create the data for a chunk packet.
use std::io::Cursor;

use crate::chunk::level_chunk::LevelChunk;

/// A struct that contains the data for a chunk packet.
/// This is equivalent to `ClientboundLevelChunkPacketData` in vanilla.
///
/// We have it in world because protocol has no access to world.
pub struct ChunkPacketData<'a> {
    /// The chunk to create the packet data for.
    pub chunk: &'a LevelChunk,
}

impl<'a> ChunkPacketData<'a> {
    /// Creates a new `ChunkPacketData`.
    #[must_use]
    pub fn new(chunk: &'a LevelChunk) -> Self {
        Self { chunk }
    }

    /// Extracts the chunk data from the chunk.
    #[must_use]
    pub fn extract_chunk_data(&self) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());

        for section in &self.chunk.sections.sections {
            section.write(&mut buf);
        }

        buf.into_inner()
    }
}
