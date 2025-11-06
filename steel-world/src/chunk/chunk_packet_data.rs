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

    pub async fn extract_chunk_data(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        for section in &self.chunk.data.read().await.sections {
            section.write(&mut buf);
        }

        buf
    }
}
