use scc::HashIndex;
use steel_utils::{ChunkPos, locks::SteelRwLock};

use crate::section::ChunkSections;

pub mod section;

#[derive(Debug)]
pub struct ChunkData {
    pub sections: ChunkSections,
}

pub struct Level {
    pub chunks: HashIndex<ChunkPos, SteelRwLock<ChunkData>>,
}

impl Default for Level {
    fn default() -> Self {
        Self::new()
    }
}

impl Level {
    pub fn new() -> Self {
        Self {
            chunks: HashIndex::new(),
        }
    }
}
