use crate::chunk::proto_chunk::ProtoChunk;

pub trait ChunkGenerator {
    // TODO: Look into making the proto chunks be chunk holders instead, otherwise it holdsd the lock for the whole chunk for the whole generation process.

    fn create_structures(&self, proto_chunk: &mut ProtoChunk);

    fn create_biomes(&self, proto_chunk: &mut ProtoChunk);

    fn fill_from_noise(&self, proto_chunk: &mut ProtoChunk);

    fn build_surface(&self, proto_chunk: &mut ProtoChunk);

    fn apply_carvers(&self, proto_chunk: &mut ProtoChunk);

    fn apply_biome_decorations(&self, proto_chunk: &mut ProtoChunk);
}
