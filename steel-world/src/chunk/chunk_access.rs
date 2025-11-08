use steel_utils::BlockStateId;

use crate::chunk::{level_chunk::LevelChunk, proto_chunk::ProtoChunk};

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

pub enum ChunkAccess {
    Full(LevelChunk),
    Proto(ProtoChunk),
}

impl ChunkAccess {
    pub fn into_full(self) -> Self {
        match self {
            Self::Proto(proto_chunk) => Self::Full(LevelChunk::from_proto(proto_chunk)),
            Self::Full(_) => unreachable!(),
        }
    }

    pub async fn get_relative_block(
        &self,
        relative_x: usize,
        relative_y: usize,
        relative_z: usize,
    ) -> Option<BlockStateId> {
        let sections = match self {
            Self::Full(chunk) => chunk.sections.read().await,
            Self::Proto(proto_chunk) => proto_chunk.sections.read().await,
        };

        sections.get_relative_block(relative_x, relative_y, relative_z)
    }

    pub async fn set_relative_block(
        &self,
        relative_x: usize,
        relative_y: usize,
        relative_z: usize,
        value: BlockStateId,
    ) {
        let mut sections = match self {
            Self::Full(chunk) => chunk.sections.write().await,
            Self::Proto(proto_chunk) => proto_chunk.sections.write().await,
        };

        sections.set_relative_block(relative_x, relative_y, relative_z, value);
    }
}
