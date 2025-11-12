use steel_utils::ChunkPos;

use crate::chunk::{proto_chunk::ProtoChunk, section::Sections};

#[derive(Debug)]
pub struct LevelChunk {
    pub sections: Sections,
    pub pos: ChunkPos,
}

impl LevelChunk {
    pub fn from_proto(proto_chunk: ProtoChunk) -> Self {
        Self {
            sections: proto_chunk.sections,
            pos: proto_chunk.pos,
        }
    }
}
