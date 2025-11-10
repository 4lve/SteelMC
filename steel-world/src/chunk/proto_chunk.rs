use steel_utils::{ChunkPos, locks::SteelRwLock};

use crate::chunk::section::Sections;

// A chunk representing a chunk that is generating
#[derive(Debug)]
pub struct ProtoChunk {
    pub sections: SteelRwLock<Sections>,
    pub pos: ChunkPos,
}
