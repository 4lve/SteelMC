#![allow(missing_docs)]

use std::sync::Arc;

use crate::chunk::{
    chunk_access::{ChunkAccess, ChunkStatus},
    chunk_generation_task::StaticCache2D,
    chunk_generator::YieldableGuard,
    chunk_holder::ChunkHolder,
    chunk_pyramid::ChunkStep,
    proto_chunk::ProtoChunk,
    section::{ChunkSection, Sections},
    world_gen_context::WorldGenContext,
};

pub struct ChunkStatusTasks;

/// All these functions are blocking.
impl ChunkStatusTasks {
    pub fn empty(
        _context: Arc<WorldGenContext>,
        _step: &Arc<ChunkStep>,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        // TODO: Check if chunk exists on disk and load it.
        // For now, create a new empty chunk.
        let sections = (0..24) // Standard height?
            .map(|_| ChunkSection::new_empty())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        // TODO: Use upgrade_to_full if the loaded chunk is full.
        let proto_chunk = ProtoChunk {
            sections: Sections { sections },
            pos: holder.get_pos(),
        };

        //log::info!("Inserted proto chunk for {:?}", holder.get_pos());

        holder.insert_chunk(ChunkAccess::Proto(proto_chunk), ChunkStatus::Empty);
        Ok(())
    }

    /// Generates structure starts.
    ///
    /// # Panics
    /// Panics if the chunk is not at `ChunkStatus::Empty` or higher.
    pub fn generate_structure_starts(
        _context: Arc<WorldGenContext>,
        _step: &Arc<ChunkStep>,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn generate_structure_references(
        _context: Arc<WorldGenContext>,
        _step: &Arc<ChunkStep>,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn load_structure_starts(
        _context: Arc<WorldGenContext>,
        _step: &Arc<ChunkStep>,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn generate_biomes(
        _context: Arc<WorldGenContext>,
        _step: &Arc<ChunkStep>,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn generate_noise(
        context: Arc<WorldGenContext>,
        _step: &Arc<ChunkStep>,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        let chunk = holder
            .try_chunk(ChunkStatus::Biomes)
            .expect("Chunk not found at status Biomes");
        context
            .generator
            .fill_from_noise(&mut YieldableGuard::new(chunk));
        Ok(())
    }

    pub fn generate_surface(
        _context: Arc<WorldGenContext>,
        _step: &Arc<ChunkStep>,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn generate_carvers(
        _context: Arc<WorldGenContext>,
        _step: &Arc<ChunkStep>,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn generate_features(
        _context: Arc<WorldGenContext>,
        _step: &Arc<ChunkStep>,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn initialize_light(
        _context: Arc<WorldGenContext>,
        _step: &Arc<ChunkStep>,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn light(
        _context: Arc<WorldGenContext>,
        _step: &Arc<ChunkStep>,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn generate_spawn(
        _context: Arc<WorldGenContext>,
        _step: &Arc<ChunkStep>,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn full(
        _context: Arc<WorldGenContext>,
        _step: &Arc<ChunkStep>,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        //panic!("Full task");
        //log::info!("Chunk {:?} upgraded to full", holder.get_pos());
        holder.upgrade_to_full();
        Ok(())
    }
}
