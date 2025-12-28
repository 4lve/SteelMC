//! This module contains the `WorldGenContext` struct, which is used to provide context for chunk generation.

use std::sync::Arc;

use enum_dispatch::enum_dispatch;

use crate::chunk::{
    chunk_access::ChunkAccess, chunk_generator::ChunkGenerator,
    flat_chunk_generator::FlatChunkGenerator, plugin_chunk_generator::PluginChunkGeneratorWrapper,
};

#[allow(missing_docs)]
#[enum_dispatch(ChunkGenerator)]
pub enum ChunkGeneratorType {
    Flat(FlatChunkGenerator),
    Plugin(PluginChunkGeneratorWrapper),
}


/// Context for world generation.
pub struct WorldGenContext {
    /// The chunk generator to use.
    pub generator: Arc<ChunkGeneratorType>,
    // Add other fields as needed:
    // pub level: ServerLevel,
    // pub structure_manager: StructureTemplateManager,
    // pub light_engine: ThreadedLevelLightEngine,
    // pub main_thread_executor: Executor,
    // pub unsaved_listener: UnsavedListener,
}
