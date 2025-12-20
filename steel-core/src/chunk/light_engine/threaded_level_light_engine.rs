//! Threaded light engine with task queue system.
//!
//! This extends the base `LightEngine` with asynchronous task scheduling and batched execution.
//! Tasks are divided into `PRE_UPDATE` (setup) and `POST_UPDATE` (completion) phases.

use std::{sync::Arc, time::Instant};

use steel_registry::{blocks::BlockRegistry, vanilla_blocks};
use steel_utils::{BlockPos, BlockStateId, ChunkPos, locks::SyncMutex, math::Vector3};

use crate::chunk::{
    chunk_access::{ChunkAccess, ChunkStatus},
    chunk_generation_task::StaticCache2D,
    chunk_generator::ChunkGuard,
    chunk_holder::ChunkHolder,
    paletted_container::PalettedContainer,
    section::Sections,
};

use super::{
    base::{BoundaryCrossing, CenterChunkLightAccess, LightEngine},
    queue_entry::QueueEntry,
};

/// Synchronous center-chunk-only light access for lock-free propagation.
///
/// Returns `None` when accessing positions outside the center chunk bounds,
/// allowing boundary crossings to be collected for batch processing.
struct CenterOnlyChunkAccess<'a> {
    /// The center chunk position.
    chunk_pos: ChunkPos,
    /// Reference to the center chunk's sections.
    sections: &'a Sections,
    /// Minimum Y coordinate of the world.
    chunk_min_y: i32,
    /// Block registry for block properties.
    block_registry: Arc<BlockRegistry>,
}

impl<'a> CenterOnlyChunkAccess<'a> {
    fn new(
        chunk_pos: ChunkPos,
        sections: &'a Sections,
        chunk_min_y: i32,
        block_registry: Arc<BlockRegistry>,
    ) -> Self {
        Self {
            chunk_pos,
            sections,
            chunk_min_y,
            block_registry,
        }
    }

    /// Checks if a position is within the center chunk.
    fn is_in_center_chunk(&self, pos: BlockPos) -> bool {
        let chunk_x = pos.0.x >> 4;
        let chunk_z = pos.0.z >> 4;
        chunk_x == self.chunk_pos.0.x && chunk_z == self.chunk_pos.0.y
    }

    /// Converts world position to chunk-relative coordinates.
    fn to_relative_coords(&self, pos: BlockPos) -> Option<(usize, usize, usize)> {
        if !self.is_in_center_chunk(pos) {
            return None;
        }

        let rel_x = (pos.0.x & 15) as usize;
        let rel_y = (pos.0.y - self.chunk_min_y) as usize;
        let rel_z = (pos.0.z & 15) as usize;

        Some((rel_x, rel_y, rel_z))
    }
}

impl CenterChunkLightAccess for CenterOnlyChunkAccess<'_> {
    fn center_chunk_pos(&self) -> ChunkPos {
        self.chunk_pos
    }

    fn get_light(&self, pos: BlockPos) -> Option<u8> {
        let (rel_x, rel_y, rel_z) = self.to_relative_coords(pos)?;

        let section_idx = rel_y / 16;
        let section_y = rel_y % 16;
        let light_section_idx = section_idx + 1; // +1 for padding

        if light_section_idx < self.sections.block_light.len() {
            Some(
                self.sections.block_light[light_section_idx]
                    .read()
                    .get(rel_x, section_y, rel_z),
            )
        } else {
            Some(0)
        }
    }

    fn set_light(&self, pos: BlockPos, level: u8) -> bool {
        let Some((rel_x, rel_y, rel_z)) = self.to_relative_coords(pos) else {
            return false;
        };

        let section_idx = rel_y / 16;
        let section_y = rel_y % 16;
        let light_section_idx = section_idx + 1; // +1 for padding

        if light_section_idx < self.sections.block_light.len() {
            self.sections.block_light[light_section_idx]
                .write()
                .set(rel_x, section_y, rel_z, level);
            true
        } else {
            false
        }
    }

    fn get_block_state(&self, pos: BlockPos) -> Option<BlockStateId> {
        let (rel_x, rel_y, rel_z) = self.to_relative_coords(pos)?;
        self.sections.get_relative_block(rel_x, rel_y, rel_z)
    }

    fn is_empty_shape(&self, pos: BlockPos) -> Option<bool> {
        let block_state = self.get_block_state(pos)?;

        if let Some(block) = self.block_registry.by_state_id(block_state) {
            Some(!block.behaviour.has_collision)
        } else {
            Some(true)
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
    light_engine: Arc<SyncMutex<LightEngine>>,
    /// Queued tasks waiting to be executed.
    light_tasks: Arc<SyncMutex<Vec<LightTask>>>,
    /// Block registry for block properties.
    block_registry: Arc<BlockRegistry>,
    /// 2-element LRU cache for chunk access.
    chunk_cache: Arc<SyncMutex<super::chunk_cache::ChunkCache>>,
}

impl ThreadedLevelLightEngine {
    /// Creates a new threaded light engine.
    #[must_use]
    pub fn new(block_registry: Arc<BlockRegistry>) -> Self {
        Self {
            light_engine: Arc::new(SyncMutex::new(LightEngine::new())),
            light_tasks: Arc::new(SyncMutex::new(Vec::new())),
            block_registry,
            chunk_cache: Arc::new(SyncMutex::new(super::chunk_cache::ChunkCache::new())),
        }
    }

    /// Clears the chunk cache.
    pub fn clear_chunk_cache(&self) {
        let mut cache = self.chunk_cache.lock();
        cache.clear();
    }

    /// Disables chunk caching (for debugging/testing).
    pub fn disable_chunk_cache(&self) {
        let mut cache = self.chunk_cache.lock();
        cache.disable();
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
        chunk: arc_swap::Guard<Option<std::sync::Arc<ChunkAccess>>>,
        light_enabled: bool,
    ) -> Result<(), anyhow::Error> {
        let chunk_guard = ChunkGuard::new(chunk);
        let chunk_pos = match &*chunk_guard {
            ChunkAccess::Proto(proto) => proto.pos,
            ChunkAccess::Full(full) => full.pos,
        };
        drop(chunk_guard);

        // PRE_UPDATE task: Mark non-empty sections for lighting
        self.add_task(chunk_pos, TaskType::PreUpdate, move || {
            // TODO: Implement section scanning when section status tracking is added
        });

        // POST_UPDATE task: Enable lighting and configure data retention
        self.add_task(chunk_pos, TaskType::PostUpdate, move || {
            // TODO: Implement when light enable/disable tracking is added
            let _ = light_enabled;
            let _ = chunk_pos;
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
    }

    /// Runs all queued light tasks and propagates light.
    ///
    /// Execution order:
    /// 1. Execute all `PRE_UPDATE` tasks
    /// 2. Run light propagation (`run_light_updates`)
    /// 3. Execute all `POST_UPDATE` tasks
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

    /// Propagates light throughout a chunk with cross-chunk support.
    ///
    /// This method:
    /// 1. Initializes sky light (fills air columns from top)
    /// 2. Scans for block light sources (blocks with luminance > 0)
    /// 3. Enqueues all light sources for propagation
    /// 4. Runs the flood-fill light propagation algorithm with cross-chunk access
    ///
    /// Unlike `light_chunk`, this version uses a cache of neighboring chunks to enable
    /// light propagation across chunk boundaries.
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::unused_async)]
    pub async fn light_chunk_with_cache(
        &self,
        chunk_guard: &ChunkGuard,
        cache: &StaticCache2D<Arc<ChunkHolder>>,
        center_holder: &Arc<ChunkHolder>,
        _light_enabled: bool,
    ) -> Result<(), anyhow::Error> {
        use rustc_hash::FxHashMap;

        let overall_start = Instant::now();

        let (chunk_pos, sections) = match &**chunk_guard {
            ChunkAccess::Proto(proto_chunk) => (proto_chunk.pos, &proto_chunk.sections),
            ChunkAccess::Full(level_chunk) => (level_chunk.pos, &level_chunk.sections),
        };

        let num_sections = sections.sections.len();
        let num_light_sections = sections.block_light.len();
        let chunk_min_y = -64;

        // ===== STEP 1: Initialize sky light with empty section optimization =====
        let mut sky_engine = super::sky_light_engine::SkyLightEngine::new();
        sky_engine.propagate_from_empty_sections(chunk_pos, sections, chunk_min_y);

        // Update sky light sources after propagation
        sections
            .sky_light_sources
            .write()
            .update_from_chunk_sections(&sections.sections, chunk_min_y);

        // ===== STEP 2: Scan for block light sources and enqueue =====
        // Create a new light engine instance for this chunk to avoid lock contention
        let mut engine = super::base::LightEngine::new();

        // Scan all blocks in the chunk for light emitters
        for section_idx in 0..num_sections {
            let section = &sections.sections[section_idx];
            let section_y = chunk_min_y + (section_idx as i32 * 16);

            // Optimization: Skip sections that are entirely air (common case)
            // Air blocks have no luminance, so we can skip the entire section
            let section_guard = section.read();
            if let PalettedContainer::Homogeneous(block_state) = &section_guard.states {
                let luminance = vanilla_blocks::get_block_luminance(*block_state);
                if luminance == 0 {
                    // Entire section is non-emitting, skip it
                    continue;
                } else if luminance > 0 {
                    // Entire section is the same emitting block (rare but possible)
                    // Fill entire section with that light level
                    let light_section_idx = section_idx + 1; // +1 for padding
                    let mut block_light_section = sections.block_light[light_section_idx].write();
                    for y in 0..16 {
                        for z in 0..16 {
                            for x in 0..16 {
                                block_light_section.set(x, y, z, luminance);

                                let world_y = section_y + y as i32;
                                let world_x = (chunk_pos.0.x * 16) + x as i32;
                                let world_z = (chunk_pos.0.y * 16) + z as i32;
                                let pos = BlockPos(Vector3::new(world_x, world_y, world_z));

                                let is_empty_shape = !self.has_collision(*block_state);
                                engine.enqueue_increase(
                                    pos,
                                    QueueEntry::increase_from_emission(luminance, is_empty_shape),
                                );
                            }
                        }
                    }
                    continue;
                }
            }

            // Heterogeneous section: scan each block individually
            let light_section_idx = section_idx + 1; // +1 for padding
            let mut block_light_section = sections.block_light[light_section_idx].write();

            for y in 0..16 {
                for z in 0..16 {
                    for x in 0..16 {
                        let block_state = section_guard.states.get(x, y, z);
                        let luminance = vanilla_blocks::get_block_luminance(block_state);

                        if luminance > 0 {
                            // Found a light source! Enqueue it for propagation
                            let world_y = section_y + y as i32;
                            let world_x = (chunk_pos.0.x * 16) + x as i32;
                            let world_z = (chunk_pos.0.y * 16) + z as i32;

                            let pos = BlockPos(Vector3::new(world_x, world_y, world_z));

                            // Set the light at this position
                            block_light_section.set(x, y, z, luminance);

                            // Enqueue for propagation in all directions
                            let is_empty_shape = !self.has_collision(block_state);
                            engine.enqueue_increase(
                                pos,
                                QueueEntry::increase_from_emission(luminance, is_empty_shape),
                            );
                        }
                    }
                }
            }
        }

        // ===== STEP 3: Two-phase light propagation =====

        // PHASE 1: Center chunk only (synchronous, lock-free)
        {
            let (_, sections_for_phase1) = match &**chunk_guard {
                ChunkAccess::Proto(proto_chunk) => (proto_chunk.pos, &proto_chunk.sections),
                ChunkAccess::Full(level_chunk) => (level_chunk.pos, &level_chunk.sections),
            };

            let center_access = CenterOnlyChunkAccess::new(
                chunk_pos,
                sections_for_phase1,
                chunk_min_y,
                self.block_registry.clone(),
            );

            engine.run_center_chunk_updates(&center_access);
        }

        // Mark all light sections in center chunk as changed
        for section_idx in 0..num_light_sections {
            center_holder.mark_light_storage_section_changed(section_idx as u32, false);
        }

        // PHASE 2: Iterative boundary crossing propagation
        let _phase2_start = Instant::now();
        let current_crossings = engine.take_boundary_crossings();
        let _total_crossings = current_crossings.len();

        // Pre-group crossings once to avoid regrouping on every retry
        let mut crossings_by_chunk: FxHashMap<ChunkPos, Vec<BoundaryCrossing>> =
            FxHashMap::default();

        for crossing in current_crossings {
            let target_chunk_x = crossing.pos.0.x >> 4;
            let target_chunk_z = crossing.pos.0.z >> 4;
            let target_chunk = ChunkPos(steel_utils::math::Vector2::new(
                target_chunk_x,
                target_chunk_z,
            ));

            crossings_by_chunk
                .entry(target_chunk)
                .or_default()
                .push(crossing);
        }

        let mut consecutive_failed_iterations = 0;

        while !crossings_by_chunk.is_empty() {
            let mut next_crossings_by_chunk: FxHashMap<ChunkPos, Vec<BoundaryCrossing>> =
                FxHashMap::default();
            let locked_any_this_iteration = false;

            // Sort chunks by position for deterministic lock ordering (prevents deadlocks)
            let mut sorted_chunks: Vec<_> = crossings_by_chunk.into_iter().collect();
            sorted_chunks.sort_by_key(|(chunk_pos, _)| (chunk_pos.0.x, chunk_pos.0.y));

            // Process each neighbor chunk in a single lock acquisition
            for (target_chunk, chunk_crossings) in sorted_chunks {
                // Skip the center chunk - we already processed it in Phase 1
                if target_chunk == chunk_pos {
                    continue;
                }

                let chunk_holder = cache.get(target_chunk.0.x, target_chunk.0.y);

                // Try non-blocking lock acquisition
                let chunk_lock_opt = chunk_holder.try_chunk(ChunkStatus::InitializeLight);

                let Some(chunk_lock) = chunk_lock_opt else {
                    // Chunk not ready yet - defer these crossings to next iteration
                    next_crossings_by_chunk
                        .entry(target_chunk)
                        .or_default()
                        .extend(chunk_crossings);
                    continue;
                };

                if let Some(chunk_arc) = chunk_lock.as_ref() {
                    let sections = match chunk_arc.as_ref() {
                        ChunkAccess::Proto(proto) => &proto.sections,
                        ChunkAccess::Full(full) => &full.sections,
                    };

                    // Create a temporary light engine for this neighbor chunk
                    let mut neighbor_engine = LightEngine::new();

                    // Process all crossings for this chunk: set light and enqueue for propagation
                    for crossing in chunk_crossings {
                        let rel_x = (crossing.pos.0.x & 15) as usize;
                        let rel_y = (crossing.pos.0.y - chunk_min_y) as usize;
                        let rel_z = (crossing.pos.0.z & 15) as usize;

                        let section_idx = rel_y / 16;
                        let section_y = rel_y % 16;
                        let light_section_idx = section_idx + 1;

                        if light_section_idx < sections.block_light.len() {
                            let current_light = sections.block_light[light_section_idx]
                                .read()
                                .get(rel_x, section_y, rel_z);
                            let new_light = crossing.entry.level();

                            if new_light > current_light {
                                sections.block_light[light_section_idx]
                                    .write()
                                    .set(rel_x, section_y, rel_z, new_light);
                                chunk_holder.mark_light_storage_section_changed(
                                    light_section_idx as u32,
                                    false,
                                );

                                // Enqueue for propagation within this neighbor chunk
                                neighbor_engine.enqueue_increase(crossing.pos, crossing.entry);
                            }
                        }
                    }

                    // Propagate within this neighbor chunk (using interior mutability)
                    if neighbor_engine.has_work() {
                        let neighbor_access = CenterOnlyChunkAccess::new(
                            target_chunk,
                            sections,
                            chunk_min_y,
                            self.block_registry.clone(),
                        );

                        neighbor_engine.run_center_chunk_updates(&neighbor_access);

                        // Collect boundary crossings from this neighbor for next iteration
                        let new_crossings = neighbor_engine.take_boundary_crossings();
                        for crossing in new_crossings {
                            let target_chunk_x = crossing.pos.0.x >> 4;
                            let target_chunk_z = crossing.pos.0.z >> 4;
                            let target_chunk = ChunkPos(steel_utils::math::Vector2::new(
                                target_chunk_x,
                                target_chunk_z,
                            ));

                            next_crossings_by_chunk
                                .entry(target_chunk)
                                .or_default()
                                .push(crossing);
                        }
                    }
                }
            }

            // Handle cases where we couldn't make progress
            if locked_any_this_iteration {
                consecutive_failed_iterations = 0;
            } else if !next_crossings_by_chunk.is_empty() {
                consecutive_failed_iterations += 1;
            }

            // Only yield after many consecutive failures to reduce overhead
            // Higher threshold = less context switching, faster overall
            if consecutive_failed_iterations >= 10 && !next_crossings_by_chunk.is_empty() {
                tokio::task::yield_now().await;
                consecutive_failed_iterations = 0; // Reset after yield
            }

            // Move to next iteration with newly discovered crossings
            crossings_by_chunk = next_crossings_by_chunk;
        }

        let total_duration = overall_start.elapsed();
        log::info!("Light propagation for chunk {chunk_pos:?} completed in {total_duration:?}");

        Ok(())
    }

    /// Helper to check if a block has collision (inverse of `is_empty_shape`).
    fn has_collision(&self, block_state: BlockStateId) -> bool {
        if let Some(block) = self.block_registry.by_state_id(block_state) {
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
