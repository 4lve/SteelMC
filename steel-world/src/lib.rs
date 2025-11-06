use crate::chunk::chunk_access::ChunkHolder;
use scc::HashIndex;
use steel_utils::ChunkPos;

pub mod chunk;
pub mod player;
pub mod server;
pub mod world;

pub struct Level {
    pub chunks: ChunkMap,
}

impl Level {
    pub fn create() -> Self {
        Self {
            chunks: ChunkMap::new(),
        }
    }
}

pub struct ChunkMap {
    pub chunks: HashIndex<ChunkPos, ChunkHolder>,
}

impl Default for ChunkMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkMap {
    pub fn new() -> Self {
        Self {
            chunks: HashIndex::new(),
        }
    }
}
