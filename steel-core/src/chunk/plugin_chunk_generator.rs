//! Plugin chunk generator wrapper.
//!
//! This module provides a wrapper that allows plugin-provided chunk generators
//! to be used with the internal chunk generation system.

use abi_stable::{sabi_trait::TD_Opaque, std_types::RBox};
use steel_plugin_api::{
    ChunkAccessTrait, ChunkAccessTrait_TO, FfiBlockStateId, FfiChunkPos, PluginChunkGenerator_TO,
};

use crate::chunk::{chunk_access::ChunkAccess, chunk_generator::ChunkGenerator};

/// A wrapper that adapts a plugin chunk generator to the internal `ChunkGenerator` trait.
pub struct PluginChunkGeneratorWrapper {
    /// The plugin-provided generator.
    generator: PluginChunkGenerator_TO<'static, RBox<()>>,
}

impl PluginChunkGeneratorWrapper {
    /// Creates a new wrapper around a plugin generator.
    #[must_use]
    pub fn new(generator: PluginChunkGenerator_TO<'static, RBox<()>>) -> Self {
        Self { generator }
    }

    /// Returns the name of the generator.
    #[must_use]
    pub fn name(&self) -> String {
        self.generator.name().to_string()
    }
}

/// Wrapper that implements ChunkAccessTrait for ChunkAccess.
struct ChunkAccessWrapper<'a> {
    chunk: &'a ChunkAccess,
}

impl<'a> ChunkAccessWrapper<'a> {
    fn new(chunk: &'a ChunkAccess) -> Self {
        Self { chunk }
    }
}

impl ChunkAccessTrait for ChunkAccessWrapper<'_> {
    fn set_relative_block(&self, x: usize, y: usize, z: usize, block: FfiBlockStateId) {
        self.chunk
            .set_relative_block(x, y, z, steel_utils::BlockStateId(block.0));
    }

    fn get_relative_block(&self, x: usize, y: usize, z: usize) -> FfiBlockStateId {
        self.chunk
            .get_relative_block(x, y, z)
            .map_or(FfiBlockStateId(0), |b| FfiBlockStateId(b.0))
    }

    fn pos(&self) -> FfiChunkPos {
        let pos = self.chunk.pos();
        FfiChunkPos {
            x: pos.0.x,
            z: pos.0.y,
        }
    }
}

/// Creates an FFI-safe chunk access trait object from a ChunkAccess reference.
fn create_ffi_chunk_access(chunk: &ChunkAccess) -> ChunkAccessTrait_TO<'_, RBox<()>> {
    let wrapper = ChunkAccessWrapper::new(chunk);
    ChunkAccessTrait_TO::from_value(wrapper, TD_Opaque)
}

impl ChunkGenerator for PluginChunkGeneratorWrapper {
    fn create_structures(&self, chunk: &ChunkAccess) {
        let ffi_chunk = create_ffi_chunk_access(chunk);
        self.generator.create_structures(ffi_chunk);
    }

    fn create_biomes(&self, chunk: &ChunkAccess) {
        let ffi_chunk = create_ffi_chunk_access(chunk);
        self.generator.create_biomes(ffi_chunk);
    }

    fn fill_from_noise(&self, chunk: &ChunkAccess) {
        let ffi_chunk = create_ffi_chunk_access(chunk);
        self.generator.fill_from_noise(ffi_chunk);
    }

    fn build_surface(&self, chunk: &ChunkAccess) {
        let ffi_chunk = create_ffi_chunk_access(chunk);
        self.generator.build_surface(ffi_chunk);
    }

    fn apply_carvers(&self, chunk: &ChunkAccess) {
        let ffi_chunk = create_ffi_chunk_access(chunk);
        self.generator.apply_carvers(ffi_chunk);
    }

    fn apply_biome_decorations(&self, chunk: &ChunkAccess) {
        let ffi_chunk = create_ffi_chunk_access(chunk);
        self.generator.apply_biome_decorations(ffi_chunk);
    }
}
