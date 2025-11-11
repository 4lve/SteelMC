use steel_utils::ChunkPos;

use crate::chunk::section::Sections;

// A chunk representing a chunk that is generating
#[derive(Debug)]
pub struct ProtoChunk {
    pub sections: Sections,
    pub pos: ChunkPos,
}
