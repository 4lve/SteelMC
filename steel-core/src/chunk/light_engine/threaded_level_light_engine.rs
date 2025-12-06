//! Threaded light engine with task queue system.
//!
//! This extends the base `LightEngine` with asynchronous task scheduling and batched execution.
//! Tasks are divided into `PRE_UPDATE` (setup) and `POST_UPDATE` (completion) phases.

use std::sync::Arc;

use parking_lot::{Mutex, RwLock as ParkingRwLock};
use steel_utils::{BlockPos, BlockStateId, ChunkPos};

use crate::chunk::{chunk_access::ChunkAccess, section::{ChunkSection, Sections}};

use super::base::{LightChunkAccess, LightEngine};

/// Light chunk access implementation for single-chunk operations (no neighbor access).
///
/// This is used during chunk generation when neighbors may not be loaded yet.
/// Light propagation is limited to within the chunk boundaries.
struct SingleChunkLightAccess<'a> {
    sections: &'a mut Sections,
    chunk_pos: ChunkPos,
    chunk_min_y: i32,
}

impl<'a> SingleChunkLightAccess<'a> {
    fn new(sections: &'a mut Sections, chunk_pos: ChunkPos, chunk_min_y: i32) -> Self {
        Self {
            sections,
            chunk_pos,
            chunk_min_y,
        }
    }

    /// Converts world block position to chunk-relative coordinates.
    /// Returns None if position is outside this chunk.
    fn world_to_chunk_relative(&self, pos: BlockPos) -> Option<(usize, usize, usize)> {
        let chunk_min_x = self.chunk_pos.0.x * 16;
        let chunk_min_z = self.chunk_pos.0.y * 16;

        let rel_x = pos.0.x - chunk_min_x;
        let rel_y = pos.0.y - self.chunk_min_y;
        let rel_z = pos.0.z - chunk_min_z;

        if rel_x >= 0 && rel_x < 16 && rel_z >= 0 && rel_z < 16 && rel_y >= 0 {
            Some((rel_x as usize, rel_y as usize, rel_z as usize))
        } else {
            None
        }
    }
}

impl<'a> LightChunkAccess for SingleChunkLightAccess<'a> {
    fn get_light(&self, pos: BlockPos) -> u8 {
        let Some((rel_x, rel_y, rel_z)) = self.world_to_chunk_relative(pos) else {
            return 0; // Out of bounds = no light
        };

        let section_idx = rel_y / 16;
        let section_y = rel_y % 16;
        let light_section_idx = section_idx + 1; // +1 for padding

        if light_section_idx < self.sections.block_light.len() {
            self.sections.block_light[light_section_idx].get(rel_x, section_y, rel_z)
        } else {
            0
        }
    }

    fn set_light(&mut self, pos: BlockPos, level: u8) {
        let Some((rel_x, rel_y, rel_z)) = self.world_to_chunk_relative(pos) else {
            return; // Out of bounds = ignore
        };

        let section_idx = rel_y / 16;
        let section_y = rel_y % 16;
        let light_section_idx = section_idx + 1; // +1 for padding

        if light_section_idx < self.sections.block_light.len() {
            self.sections.block_light[light_section_idx].set(rel_x, section_y, rel_z, level);
        }
    }

    fn get_block_state(&self, pos: BlockPos) -> BlockStateId {
        let Some((rel_x, rel_y, rel_z)) = self.world_to_chunk_relative(pos) else {
            return BlockStateId(0); // Out of bounds = air
        };

        self.sections
            .get_relative_block(rel_x, rel_y, rel_z)
            .unwrap_or(BlockStateId(0))
    }

    fn is_empty_shape(&self, pos: BlockPos) -> bool {
        use steel_registry::REGISTRY;

        let block_state = self.get_block_state(pos);
        if let Some(block) = REGISTRY.blocks.by_state_id(block_state) {
            !block.behaviour.has_collision
        } else {
            true // Unknown blocks treated as empty
        }
    }
}

/// Task type for light engine operations.
///
/// Tasks are executed in a specific order to ensure correct light propagation:
/// 1. All `PRE_UPDATE` tasks run first (setup, marking sections)
/// 2. Light propagation runs (`run_light_updates`)
/// 3. All `POST_UPDATE` tasks run last (completion, futures)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    /// Tasks executed before light propagation (setup phase).
    PreUpdate,
    /// Tasks executed after light propagation (completion phase).
    PostUpdate,
}

/// A light engine task with its associated type.
type LightTask = (TaskType, Box<dyn FnOnce() + Send>);

/// Multi-threaded light engine that batches and schedules lighting operations.
///
/// This engine maintains:
/// - A base `LightEngine` for light propagation
/// - A task queue for `PRE_UPDATE` and `POST_UPDATE` operations
/// - Section status tracking for which sections need lighting
///
/// # Architecture
///
/// The engine follows vanilla Minecraft's pattern:
/// 1. Public methods (like `initialize_light`) create tasks
/// 2. Tasks are queued and batched
/// 3. When batch size is reached or forced, tasks execute:
///    - `PRE_UPDATE` tasks (mark sections, queue changes)
///    - Light propagation (`run_light_updates`)
///    - `POST_UPDATE` tasks (set flags, complete futures)
pub struct ThreadedLevelLightEngine {
    /// The base light engine for propagation.
    light_engine: Arc<Mutex<LightEngine>>,
    /// Queued tasks waiting to be executed.
    light_tasks: Arc<Mutex<Vec<LightTask>>>,
}

impl ThreadedLevelLightEngine {
    /// Creates a new threaded light engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            light_engine: Arc::new(Mutex::new(LightEngine::new())),
            light_tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Initializes lighting for a chunk.
    ///
    /// This method follows vanilla's approach:
    /// 1. `PRE_UPDATE` task: Scans sections and marks non-empty ones
    /// 2. `POST_UPDATE` task: Enables lighting and configures data retention
    ///
    /// **Important**: This does NOT set light values. It only:
    /// - Identifies which sections need lighting
    /// - Marks them for the light engine
    /// - Enables lighting for the chunk
    ///
    /// Actual light propagation happens later in the LIGHT chunk status.
    pub fn initialize_light(
        &self,
        chunk: &ParkingRwLock<Option<ChunkAccess>>,
        light_enabled: bool,
    ) -> Result<(), anyhow::Error> {
        use crate::chunk::chunk_generator::ChunkGuard;

        let chunk_guard = ChunkGuard::new(chunk);
        let chunk_pos = match &*chunk_guard {
            ChunkAccess::Proto(proto) => proto.pos,
            ChunkAccess::Full(full) => full.pos,
        };
        drop(chunk_guard);

        // PRE_UPDATE task: Mark non-empty sections for lighting
        self.add_task(chunk_pos, TaskType::PreUpdate, {
            // Note: We can't easily pass the chunk reference into the closure
            // For now, this is a stub - will be implemented when section status tracking is added
            move || {
                // TODO: Access chunk and scan sections
                // let chunk_guard = ChunkGuard::new(chunk);
                // let sections = match &*chunk_guard { ... };

                // TODO: Scan sections and mark non-empty ones
                // for (i, section) in sections.sections.iter().enumerate() {
                //     if !Self::is_section_empty(section) {
                //         super.updateSectionStatus(SectionPos.of(chunkPos, section_y), false);
                //     }
                // }
            }
        });

        // POST_UPDATE task: Enable lighting and configure data retention
        self.add_task(chunk_pos, TaskType::PostUpdate, {
            move || {
                // TODO: Implement set_light_enabled and retain_data
                // super.setLightEnabled(chunkPos, lightEnabled);
                // super.retainData(chunkPos, false);
                let _ = light_enabled;
                let _ = chunk_pos;
            }
        });

        Ok(())
    }

    /// Adds a task to the task queue.
    fn add_task<F>(&self, _chunk_pos: ChunkPos, task_type: TaskType, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let mut tasks = self.light_tasks.lock();
        tasks.push((task_type, Box::new(task)));

        // TODO: Implement batch scheduling
        // In vanilla, tasks accumulate until batch size reaches 1000 or tryScheduleUpdate() is called
        // For now, we'll just queue them
    }

    /// Checks if a section is empty (all air blocks).
    #[allow(dead_code)] // Stubbed for future use
    fn is_section_empty(section: &ChunkSection) -> bool {
        // A section is empty if it only contains air (BlockStateId 0)
        match &section.states {
            crate::chunk::paletted_container::PalettedContainer::Homogeneous(id) => id.0 == 0,
            crate::chunk::paletted_container::PalettedContainer::Heterogeneous(_) => {
                // If it's heterogeneous, it has different block types, so not empty
                false
            }
        }
    }

    /// Runs all queued light tasks and propagates light.
    ///
    /// Execution order:
    /// 1. Execute all `PRE_UPDATE` tasks
    /// 2. Run light propagation (`run_light_updates`)
    /// 3. Execute all `POST_UPDATE` tasks
    ///
    /// # Note
    /// This is currently a stub. In the full implementation, this would be called
    /// by the chunk task dispatcher when the batch is ready.
    pub fn run_update(&self) {
        let mut tasks = self.light_tasks.lock();
        let all_tasks = std::mem::take(&mut *tasks);
        drop(tasks);

        // Separate tasks by type
        let (pre_update_tasks, post_update_tasks): (Vec<_>, Vec<_>) = all_tasks
            .into_iter()
            .partition(|(task_type, _)| *task_type == TaskType::PreUpdate);

        // Execute PRE_UPDATE tasks
        for (_, task) in pre_update_tasks {
            task();
        }

        // Run light propagation
        let mut engine = self.light_engine.lock();
        engine.run_light_updates();
        drop(engine);

        // Execute POST_UPDATE tasks
        for (_, task) in post_update_tasks {
            task();
        }
    }

    /// Propagates light throughout a chunk.
    ///
    /// This method:
    /// 1. Initializes sky light (fills air columns from top)
    /// 2. Scans for block light sources (blocks with luminance > 0)
    /// 3. Enqueues all light sources for propagation
    /// 4. Runs the flood-fill light propagation algorithm
    pub fn light_chunk(
        &self,
        chunk: &ParkingRwLock<Option<ChunkAccess>>,
        _light_enabled: bool,
    ) -> Result<(), anyhow::Error> {
        use crate::chunk::{chunk_generator::ChunkGuard, light_storage::LightStorage};
        use steel_registry::vanilla_blocks;
        use steel_utils::{BlockPos, BlockStateId, math::Vector3};

        use super::queue_entry::QueueEntry;

        let mut chunk_guard = ChunkGuard::new(chunk);
        let (chunk_pos, sections) = match &mut *chunk_guard {
            ChunkAccess::Proto(proto_chunk) => (proto_chunk.pos, &mut proto_chunk.sections),
            ChunkAccess::Full(level_chunk) => (level_chunk.pos, &mut level_chunk.sections),
        };

        let num_sections = sections.sections.len();
        let chunk_min_y = -64; // TODO: Get from world height settings

        // ===== STEP 1: Initialize sky light (simple vertical fill) =====
        let mut current_section = 0;

        // Scan from top to bottom to find sections that are all air
        for index in (0..num_sections + 2).rev() {
            if index == 0 {
                sections.sky_light[index] = LightStorage::new_empty();
            } else if index == num_sections + 1 {
                sections.sky_light[index] = LightStorage::new_filled(15);
            } else if let Some(section) = sections.sections.get(index - 1) {
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
                    break;
                }
            }
        }

        let start_section = if current_section > 0 {
            current_section - 1
        } else {
            0
        };

        // Fill sky light columns for non-empty sections
        for x in 0..16 {
            for z in 0..16 {
                for section_idx in (0..=start_section).rev() {
                    if section_idx == 0 {
                        continue;
                    }

                    let actual_section_idx = section_idx - 1;
                    if actual_section_idx >= num_sections {
                        continue;
                    }

                    let section = &sections.sections[actual_section_idx];

                    for y in (0..16).rev() {
                        let block_state = section.states.get(x, y, z);
                        let is_air = block_state == BlockStateId(0);

                        if is_air {
                            sections.sky_light[section_idx].set(x, y, z, 15);
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        // ===== STEP 2: Scan for block light sources and enqueue =====
        let mut engine = self.light_engine.lock();

        // Scan all blocks in the chunk for light emitters
        for section_idx in 0..num_sections {
            let section = &sections.sections[section_idx];
            let section_y = chunk_min_y + (section_idx as i32 * 16);

            for y in 0..16 {
                for z in 0..16 {
                    for x in 0..16 {
                        let block_state = section.states.get(x, y, z);
                        let luminance = vanilla_blocks::get_block_luminance(block_state);

                        if luminance > 0 {
                            // Found a light source! Enqueue it for propagation
                            let world_y = section_y + y as i32;
                            let world_x = (chunk_pos.0.x * 16) + x as i32;
                            let world_z = (chunk_pos.0.y * 16) + z as i32;

                            let pos = BlockPos(Vector3::new(world_x, world_y, world_z));

                            // Set the light at this position
                            let light_section_idx = section_idx + 1; // +1 for padding
                            sections.block_light[light_section_idx].set(x, y, z, luminance);

                            // Enqueue for propagation in all directions
                            let is_empty_shape = !Self::has_collision(block_state);
                            engine.enqueue_increase(
                                pos,
                                QueueEntry::increase_from_emission(luminance, is_empty_shape),
                            );
                        }
                    }
                }
            }
        }

        // ===== STEP 3: Create chunk access adapter and propagate =====
        // Create a single-chunk access adapter (doesn't access neighbors)
        let mut chunk_access = SingleChunkLightAccess::new(sections, chunk_pos, chunk_min_y);

        // Run the flood-fill light propagation algorithm
        engine.run_light_updates_with_access(&mut chunk_access);

        // Note: Lights at chunk boundaries won't propagate to neighbors
        // since SingleChunkLightAccess returns 0/air for out-of-chunk positions.
        // Cross-chunk propagation will be added later once the chunk access
        // architecture supports reading neighbor chunks during generation.

        Ok(())
    }

    /// Helper to check if a block has collision (inverse of is_empty_shape).
    fn has_collision(block_state: BlockStateId) -> bool {
        use steel_registry::REGISTRY;

        if let Some(block) = REGISTRY.blocks.by_state_id(block_state) {
            block.behaviour.has_collision
        } else {
            false // Unknown blocks treated as no collision
        }
    }

    /// Checks if there are any pending tasks or light updates.
    #[must_use]
    pub fn has_work(&self) -> bool {
        let tasks = self.light_tasks.lock();
        let has_tasks = !tasks.is_empty();
        drop(tasks);

        let engine = self.light_engine.lock();
        let has_light_work = engine.has_work();
        drop(engine);

        has_tasks || has_light_work
    }
}

impl Default for ThreadedLevelLightEngine {
    fn default() -> Self {
        Self::new()
    }
}
