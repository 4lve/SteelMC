use std::io::Cursor;

use crate::chunk::level_chunk::LevelChunk;

/// ClientboundLevelChunkPacketData in java
/// We have it in world because protocol has no access to world!
pub struct ChunkPacketData<'a> {
    pub chunk: &'a LevelChunk,
}

impl<'a> ChunkPacketData<'a> {
    pub fn new(chunk: &'a LevelChunk) -> Self {
        Self { chunk }
    }

    pub fn extract_chunk_data(&self) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());

        for section in &self.chunk.sections.sections {
            section.write(&mut buf);
        }

        buf.into_inner()
    }
}
