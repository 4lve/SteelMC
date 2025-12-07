#![allow(missing_docs)]

use std::sync::Arc;

use crate::chunk::{
    chunk_access::{ChunkAccess, ChunkStatus},
    chunk_generation_task::StaticCache2D,
    chunk_generator::ChunkGenerator,
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
        _step: &ChunkStep,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        use crate::chunk::light_storage::LightStorage;

        let sections = (0..24)
            .map(|_| ChunkSection::new_empty())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let section_count = sections.len();

        let sky_light = (0..(section_count + 2))
            .map(|_| LightStorage::new_empty())
            .collect();
        let block_light = (0..(section_count + 2))
            .map(|_| LightStorage::new_empty())
            .collect();

        // TODO: Use upgrade_to_full if the loaded chunk is full.
        let proto_chunk = ProtoChunk::new(
            Sections {
                sections: sections
                    .into_iter()
                    .map(|section| Arc::new(SyncRwLock::new(section)))
                    .collect(),
                sky_light,
                block_light,
            },
            holder.get_pos(),
        );

        holder.insert_chunk(ChunkAccess::Proto(proto_chunk), ChunkStatus::Empty);
        Ok(())
    }

    /// Generates structure starts.
    ///
    /// # Panics
    /// Panics if the chunk is not at `ChunkStatus::Empty` or higher.
    pub fn generate_structure_starts(
        _context: Arc<WorldGenContext>,
        _step: &ChunkStep,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn generate_structure_references(
        _context: Arc<WorldGenContext>,
        _step: &ChunkStep,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn load_structure_starts(
        _context: Arc<WorldGenContext>,
        _step: &ChunkStep,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn generate_biomes(
        _context: Arc<WorldGenContext>,
        _step: &ChunkStep,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn generate_noise(
        context: Arc<WorldGenContext>,
        _step: &ChunkStep,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        let chunk = holder
            .try_chunk(ChunkStatus::Biomes)
            .expect("Chunk not found at status Biomes");
        context
            .generator
            .fill_from_noise(chunk.as_ref().expect("Chunk is not loaded").as_ref());
        Ok(())
    }

    pub fn generate_surface(
        _context: Arc<WorldGenContext>,
        _step: &ChunkStep,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn generate_carvers(
        _context: Arc<WorldGenContext>,
        _step: &ChunkStep,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn generate_features(
        _context: Arc<WorldGenContext>,
        _step: &ChunkStep,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    /// Initializes lighting for the chunk.
    ///
    /// This method prepares the chunk for light propagation.
    ///
    /// # Panics
    /// Panics if the chunk is not at `ChunkStatus::Features` or higher.
    pub fn initialize_light(
        context: Arc<WorldGenContext>,
        _step: &ChunkStep,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        let chunk = holder
            .try_chunk(ChunkStatus::Features)
            .expect("Chunk not found at status Features");

        let is_lighted = true;
        context.light_engine.initialize_light(chunk, is_lighted)?;

        Ok(())
    }

    /// Propagates light throughout the chunk.
    ///
    /// # Panics
    /// Panics if the chunk is not at `ChunkStatus::InitializeLight` or higher.
    pub fn light(
        context: Arc<WorldGenContext>,
        _step: &ChunkStep,
        cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        let chunk = holder
            .try_chunk(ChunkStatus::InitializeLight)
            .expect("Chunk not found at status InitializeLight");

        let is_lighted = true;
        let mut guard = ChunkGuard::new(chunk);

        // Block on the async light propagation
        // This is safe because the Tokio runtime has its own thread pool
        context.runtime_handle.block_on(
            context
                .light_engine
                .light_chunk_with_cache(&mut guard, cache, is_lighted),
        )?;

        Ok(())
    }

    pub fn generate_spawn(
        _context: Arc<WorldGenContext>,
        _step: &ChunkStep,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn full(
        _context: Arc<WorldGenContext>,
        _step: &ChunkStep,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        holder.upgrade_to_full();
        Ok(())
    }
}
