use std::sync::Arc;

use crate::chunk::{level_chunk::LevelChunk, section::ChunkSection};

// A chunk represeting a chunk that is generating
#[derive(Debug)]
pub struct ProtoChunk {
    pub sections: Box<[ChunkSection]>,
}

pub enum ChunkAccses {
    Full(Arc<LevelChunk>),
    Proto(Arc<ProtoChunk>),
}

pub enum ChunkStatus {
    Empty,
    StructureStarts,
    StructureReferences,
    Biomes,
    Noise,
    Surface,
    Carvers,
    Features,
    InitializeLight,
    Light,
    Spawn,
    Full,
}
