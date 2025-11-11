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
        let mut writer = Cursor::new(Vec::new());

        for section in &self.chunk.sections.blocking_read().sections {
            section.write(&mut writer);
        }

        writer.into_inner()
    }
}
