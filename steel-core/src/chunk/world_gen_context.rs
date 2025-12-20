//! This module contains the `WorldGenContext` struct, which is used to provide context for chunk generation.

use std::sync::Arc;

use enum_dispatch::enum_dispatch;

use crate::chunk::{
    chunk_access::ChunkAccess, chunk_generator::ChunkGenerator,
    flat_chunk_generator::FlatChunkGenerator, light_engine::ThreadedLevelLightEngine,
};

#[allow(missing_docs)]
#[enum_dispatch(ChunkGenerator)]
pub enum ChunkGeneratorType {
    Flat(FlatChunkGenerator),
    //Custom(Box<dyn ChunkGenerator>),
}

/// Context for world generation.
pub struct WorldGenContext {
    /// The chunk generator to use.
    pub generator: Arc<ChunkGeneratorType>,
    /// The light engine for chunk lighting.
    pub light_engine: Arc<ThreadedLevelLightEngine>,
    /// Tokio runtime handle for async operations in sync contexts.
    pub runtime_handle: tokio::runtime::Handle,
    // Add other fields as needed:
    // pub level: ServerLevel,
    // pub structure_manager: StructureTemplateManager,
    // pub main_thread_executor: Executor,
    // pub unsaved_listener: UnsavedListener,
}
