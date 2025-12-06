#![allow(missing_docs)]

use std::sync::Arc;

use crate::chunk::{
    chunk_access::{ChunkAccess, ChunkStatus},
    chunk_generation_task::StaticCache2D,
    chunk_generator::ChunkGenerator,
    chunk_holder::ChunkHolder,
    chunk_pyramid::ChunkStep,
    light_storage::LightStorage,
    proto_chunk::ProtoChunk,
    section::{ChunkSection, Sections},
    world_gen_context::WorldGenContext,
};
use steel_utils::BlockStateId;

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

        // TODO: Check if chunk exists on disk and load it.
        // For now, create a new empty chunk.
        let sections = (0..24) // Standard height?
            .map(|_| ChunkSection::new_empty())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let section_count = sections.len();

        // Initialize light storage (sections.len() + 2 for padding above and below)
        // Sky light starts at 0, block light starts at 0
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
    /// # Panics
    /// Panics if the chunk is not at `ChunkStatus::Features` or higher.
    pub fn initialize_light(
        _context: Arc<WorldGenContext>,
        _step: &ChunkStep,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        let chunk = holder
            .try_chunk(ChunkStatus::Features)
            .expect("Chunk not found at status Features");

        let mut chunk_guard = ChunkGuard::new(chunk);
        let sections = match &mut *chunk_guard {
            ChunkAccess::Proto(proto_chunk) => &mut proto_chunk.sections,
            ChunkAccess::Full(level_chunk) => &mut level_chunk.sections,
        };

        let num_sections = sections.sections.len();
        debug_assert_eq!(sections.sky_light.len(), num_sections + 2);
        debug_assert_eq!(sections.block_light.len(), num_sections + 2);

        // Block light: stays at 0 (already initialized in empty stage)

        // Sky light: Fill homogeneous sections from top down until we hit non-air blocks
        let mut current_section = 0;

        // Scan from top to bottom to find sections that are all air
        for index in (0..num_sections + 2).rev() {
            // First section (bottom padding) is always empty (0)
            if index == 0 {
                sections.sky_light[index] = LightStorage::new_empty();
            } else if index == num_sections + 1 {
                // Top padding is always full light
                sections.sky_light[index] = LightStorage::new_filled(15);
            } else if let Some(section) = sections.sections.get(index - 1) {
                // Check if section is all air (homogeneous with value 0)
                let is_all_air = match &section.states {
                    crate::chunk::paletted_container::PalettedContainer::Homogeneous(id) => {
                        *id == BlockStateId(0)
                    }
                    crate::chunk::paletted_container::PalettedContainer::Heterogeneous(_) => false,
                };

                if is_all_air {
                    sections.sky_light[index] = LightStorage::new_filled(15);
                    current_section = index;
                } else {
                    // Hit a section with blocks, stop filling homogeneous
                    break;
                }
            }
        }

        // Now do per-block light propagation for remaining sections
        // current_section is the highest section with all air (or 0 if none)
        let start_section = if current_section > 0 {
            current_section - 1
        } else {
            0
        };

        for x in 0..16 {
            for z in 0..16 {
                // Iterate from top section down to bottom
                for section_idx in (0..=start_section).rev() {
                    if section_idx == 0 {
                        // Bottom padding, skip
                        continue;
                    }

                    let actual_section_idx = section_idx - 1;
                    if actual_section_idx >= num_sections {
                        continue;
                    }

                    let section = &sections.sections[actual_section_idx];

                    // Iterate through y in this section from top to bottom
                    for y in (0..16).rev() {
                        let block_state = section.states.get(x, y, z);
                        let is_air = block_state == BlockStateId(0);

                        if is_air {
                            // Air block: set full light (15)
                            sections.sky_light[section_idx].set(x, y, z, 15);
                        } else {
                            // Hit a non-air block: stop propagating down this column
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn light(
        _context: Arc<WorldGenContext>,
        _step: &ChunkStep,
        _cache: &Arc<StaticCache2D<Arc<ChunkHolder>>>,
        _holder: Arc<ChunkHolder>,
    ) -> Result<(), anyhow::Error> {
        // TODO: Implement light propagation
        // Now that all light sources are in place, propagate light throughout the chunk.
        // (Block light sources are blocks that have non-zero light emission values.)
        // The sky light was initialized to 15 from the top down in the initialize_light step.
        // Now, we just need to propagate light from these sources.
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
        //panic!("Full task");
        //log::info!("Chunk {:?} upgraded to full", holder.get_pos());
        holder.upgrade_to_full();
        Ok(())
    }
}
