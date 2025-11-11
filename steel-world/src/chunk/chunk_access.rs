use steel_utils::BlockStateId;

use crate::chunk::{level_chunk::LevelChunk, proto_chunk::ProtoChunk};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
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

pub enum ChunkAccses {
    Full(LevelChunk),
    Proto(ProtoChunk),
}

impl ChunkAccses {
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
            Self::Full(chunk) => &chunk.sections,
            Self::Proto(proto_chunk) => &proto_chunk.sections,
        };

        sections.get_relative_block(relative_x, relative_y, relative_z)
    }

    pub async fn set_relative_block(
        &mut self,
        relative_x: usize,
        relative_y: usize,
        relative_z: usize,
        value: BlockStateId,
    ) {
        let sections = match self {
            Self::Full(chunk) => &mut chunk.sections,
            Self::Proto(proto_chunk) => &mut proto_chunk.sections,
        };

        sections.set_relative_block(relative_x, relative_y, relative_z, value);
    }
}
