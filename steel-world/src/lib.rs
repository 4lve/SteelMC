use scc::HashIndex;
use std::sync::Arc;
use steel_utils::ChunkPos;

use crate::chunk::chunk_holder::ChunkHolder;

pub mod chunk;
pub mod config;
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
    pub chunks: HashIndex<ChunkPos, Arc<ChunkHolder>>,
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
