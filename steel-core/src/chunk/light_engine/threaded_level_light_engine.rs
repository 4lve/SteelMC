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

/// Type of light being propagated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LightType {
    /// Block light (emitted by light-emitting blocks like torches).
    Block,
    /// Sky light (comes from the sky, propagates downward and horizontally).
    Sky,
}

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
    /// Type of light (block or sky).
    light_type: LightType,
}

impl<'a> CenterOnlyChunkAccess<'a> {
    fn new(
        chunk_pos: ChunkPos,
        sections: &'a Sections,
        chunk_min_y: i32,
        block_registry: Arc<BlockRegistry>,
        light_type: LightType,
    ) -> Self {
        Self {
            chunk_pos,
            sections,
            chunk_min_y,
            block_registry,
            light_type,
        }
    }

    /// Checks if a position is within the center chunk.
    #[inline]
    fn is_in_center_chunk(&self, pos: BlockPos) -> bool {
        let chunk_x = pos.0.x >> 4;
        let chunk_z = pos.0.z >> 4;
        chunk_x == self.chunk_pos.0.x && chunk_z == self.chunk_pos.0.y
    }

    /// Converts world position to chunk-relative coordinates.
    #[inline]
    fn to_relative_coords(&self, pos: BlockPos) -> Option<(usize, usize, usize)> {
        if !self.is_in_center_chunk(pos) {
            return None;
        }

        // Check Y bounds - position must be within valid chunk height range
        if pos.0.y < self.chunk_min_y {
            return None; // Below chunk
        }

        let rel_y = (pos.0.y - self.chunk_min_y) as usize;

        // Check if the Y position has a corresponding section
        let section_idx = rel_y / 16;
        if section_idx >= self.sections.sections.len() {
            return None; // Above chunk
        }

        let rel_x = (pos.0.x & 15) as usize;
        let rel_z = (pos.0.z & 15) as usize;

        Some((rel_x, rel_y, rel_z))
    }
}

impl CenterChunkLightAccess for CenterOnlyChunkAccess<'_> {
    #[inline]
    fn center_chunk_pos(&self) -> ChunkPos {
        self.chunk_pos
    }

    #[inline]
    fn get_light(&self, pos: BlockPos) -> Option<u8> {
        let (rel_x, rel_y, rel_z) = self.to_relative_coords(pos)?;

        let section_idx = rel_y / 16;
        let section_y = rel_y % 16;
        let light_section_idx = section_idx + 1; // +1 for padding

        let light_array = match self.light_type {
            LightType::Block => &self.sections.block_light,
            LightType::Sky => &self.sections.sky_light,
        };

        if light_section_idx < light_array.len() {
            Some(
                light_array[light_section_idx]
                    .read()
                    .get(rel_x, section_y, rel_z),
            )
        } else {
            Some(0)
        }
    }

    #[inline]
    fn set_light(&self, pos: BlockPos, level: u8) -> bool {
        let Some((rel_x, rel_y, rel_z)) = self.to_relative_coords(pos) else {
            return false;
        };

        let section_idx = rel_y / 16;
        let section_y = rel_y % 16;
        let light_section_idx = section_idx + 1; // +1 for padding

        let light_array = match self.light_type {
            LightType::Block => &self.sections.block_light,
            LightType::Sky => &self.sections.sky_light,
        };

        if light_section_idx < light_array.len() {
            light_array[light_section_idx]
                .write()
                .set(rel_x, section_y, rel_z, level);
            true
        } else {
            false
        }
    }

    #[inline]
    fn get_block_state(&self, pos: BlockPos) -> Option<BlockStateId> {
        let (rel_x, rel_y, rel_z) = self.to_relative_coords(pos)?;
        self.sections.get_relative_block(rel_x, rel_y, rel_z)
    }

    #[inline]
    fn is_empty_shape(&self, pos: BlockPos) -> Option<bool> {
        let block_state = self.get_block_state(pos)?;

        if let Some(block) = self.block_registry.by_state_id(block_state) {
            Some(!block.behaviour.has_collision)
        } else {
            Some(true)
        }
    }

    #[inline]
    fn get_neighbor_data(&self, pos: BlockPos) -> Option<(BlockStateId, u8)> {
        // Single coordinate conversion for both block state and light
        let (rel_x, rel_y, rel_z) = self.to_relative_coords(pos)?;

        // Get block state - guaranteed valid since to_relative_coords succeeded
        let block_state = self.sections.get_relative_block(rel_x, rel_y, rel_z)?;

        // Get light level using the same relative coordinates
        let section_idx = rel_y / 16;
        let section_y = rel_y % 16;
        let light_section_idx = section_idx + 1; // +1 for padding

        let light_array = match self.light_type {
            LightType::Block => &self.sections.block_light,
            LightType::Sky => &self.sections.sky_light,
        };

        let light = if light_section_idx < light_array.len() {
            light_array[light_section_idx]
                .read()
                .get(rel_x, section_y, rel_z)
        } else {
            0
        };

        Some((block_state, light))
    }
}

/// Multi-threaded light engine for chunk lighting.
///
/// This engine provides:
/// - Lock-free center-chunk propagation with boundary crossing collection
/// - Iterative cross-chunk propagation with non-blocking lock acquisition
/// - LRU chunk cache to reduce lock contention
/// - Empty section optimization for sky light
pub struct ThreadedLevelLightEngine {
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
    /// This is a placeholder for vanilla's section status tracking system.
    /// In vanilla, this method would:
    /// 1. Scan sections and mark non-empty ones for lighting
    /// 2. Enable lighting and configure data retention
    ///
    /// Currently, section status tracking is not implemented, so this method
    /// is a no-op. Actual light propagation happens in `light_chunk_with_cache`.
    pub fn initialize_light(
        &self,
        _chunk: arc_swap::Guard<Option<std::sync::Arc<ChunkAccess>>>,
        _light_enabled: bool,
    ) -> Result<(), anyhow::Error> {
        // No-op until section status tracking is implemented
        Ok(())
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

        // ===== STEP 1.5: Sky light horizontal propagation =====
        // After vertical fill, propagate sky light horizontally across chunk boundaries.
        // Uses vanilla's selective enqueue strategy: only enqueue blocks at terrain boundaries.
        //
        // Vanilla optimization: Instead of enqueueing all lit blocks (thousands), only enqueue:
        // 1. Blocks at lowestSourceY (terrain surface) - these propagate DOWN
        // 2. Blocks below neighbor's lowestSourceY - these propagate HORIZONTALLY to neighbors
        //
        // This reduces enqueued blocks from ~4,096 to ~256-512 per chunk (8-16x speedup).

        // Create a new light engine for sky light horizontal propagation
        let mut sky_engine_horizontal = super::base::LightEngine::new();

        // Compute enqueue positions in a separate scope to ensure lock is dropped early
        let enqueue_positions = {
            // Get sky light sources (heightmap) for selective enqueuing
            let sky_sources = sections.sky_light_sources.read();

            // Get neighbor chunks' sky light sources for boundary checking
            // Store as (dx, dz, heightmap_array) to avoid lifetime issues
            let mut neighbor_sources: Vec<(i32, i32, Box<[i32; 256]>)> = vec![];
            for dx in -1..=1_i32 {
                for dz in -1..=1_i32 {
                    if dx == 0 && dz == 0 {
                        continue; // Skip center chunk
                    }
                    let neighbor_chunk_pos = ChunkPos(steel_utils::math::Vector2::new(
                        chunk_pos.0.x + dx,
                        chunk_pos.0.y + dz,
                    ));
                    let neighbor_holder = cache.get(neighbor_chunk_pos.0.x, neighbor_chunk_pos.0.y);
                    if let Some(neighbor_arc) = neighbor_holder
                        .try_chunk(ChunkStatus::InitializeLight)
                        .and_then(|guard| guard.as_ref().cloned())
                    {
                        let heightmap = match neighbor_arc.as_ref() {
                            ChunkAccess::Proto(proto) => {
                                let sources = proto.sections.sky_light_sources.read();
                                // Clone the heightmap data (just 256 i32s, very cheap)
                                let mut heights = Box::new([0i32; 256]);
                                for i in 0..256 {
                                    heights[i] = sources.get(i % 16, i / 16);
                                }
                                heights
                            }
                            ChunkAccess::Full(full) => {
                                let sources = full.sections.sky_light_sources.read();
                                // Clone the heightmap data (just 256 i32s, very cheap)
                                let mut heights = Box::new([0i32; 256]);
                                for i in 0..256 {
                                    heights[i] = sources.get(i % 16, i / 16);
                                }
                                heights
                            }
                        };
                        neighbor_sources.push((dx, dz, heightmap));
                    }
                }
            }

            // Helper to get neighbor's lowestSourceY at chunk boundaries
            let get_neighbor_height = |x: usize, z: usize, dx: i32, dz: i32| -> i32 {
                for (ndx, ndz, heights) in &neighbor_sources {
                    if *ndx == dx && *ndz == dz {
                        // Calculate the corresponding column in the neighbor chunk
                        use std::cmp::Ordering;
                        let nx = match dx.cmp(&0) {
                            Ordering::Less => 15,
                            Ordering::Greater => 0,
                            Ordering::Equal => x,
                        };
                        let nz = match dz.cmp(&0) {
                            Ordering::Less => 15,
                            Ordering::Greater => 0,
                            Ordering::Equal => z,
                        };
                        let idx = nz * 16 + nx;
                        return heights[idx];
                    }
                }
                i32::MIN // No neighbor data available
            };

            // Selective enqueue: only blocks at lowestSourceY or below neighbor heights
            // Collect all enqueue positions first, then drop the lock before processing
            let mut enqueue_positions = Vec::new();
            for z in 0..16 {
                for x in 0..16 {
                    let lowest_source_y = sky_sources.get(x, z);

                    // Skip if no sky light in this column
                    if lowest_source_y == i32::MIN {
                        continue;
                    }

                    // Get neighbor heights for boundary checking (only for edge columns)
                    let north_height = if z == 0 {
                        get_neighbor_height(x, z, 0, -1)
                    } else {
                        i32::MIN
                    };
                    let south_height = if z == 15 {
                        get_neighbor_height(x, z, 0, 1)
                    } else {
                        i32::MIN
                    };
                    let west_height = if x == 0 {
                        get_neighbor_height(x, z, -1, 0)
                    } else {
                        i32::MIN
                    };
                    let east_height = if x == 15 {
                        get_neighbor_height(x, z, 1, 0)
                    } else {
                        i32::MIN
                    };

                    // Calculate the maximum neighbor height for this column
                    let neighbor_max = [north_height, south_height, west_height, east_height]
                        .iter()
                        .filter(|&&h| h != i32::MIN)
                        .copied()
                        .max()
                        .unwrap_or(i32::MIN);

                    // Determine Y range to check: from lowest_source_y down to just below neighbor_max
                    let min_y_to_check =
                        if neighbor_max != i32::MIN && neighbor_max > lowest_source_y {
                            lowest_source_y
                        } else if neighbor_max != i32::MIN {
                            neighbor_max - 1
                        } else {
                            lowest_source_y
                        };

                    // Only process Y positions that need to be enqueued
                    for y in min_y_to_check..=lowest_source_y {
                        // Vanilla condition: enqueue if at lowestSourceY OR below neighbor height
                        let at_lowest = y == lowest_source_y;
                        let below_neighbor = neighbor_max != i32::MIN && y < neighbor_max;

                        if at_lowest || below_neighbor {
                            enqueue_positions.push((x, y, z));
                        }
                    }
                }
            }

            drop(sky_sources);
            enqueue_positions
        }; // End of scope - sky_sources is definitely dropped here

        // Now enqueue all the collected positions
        for (x, y, z) in enqueue_positions {
            let section_idx = ((y - chunk_min_y) / 16) as usize;
            let section_y = (y - chunk_min_y) % 16;
            let light_section_idx = section_idx + 1;

            if section_idx >= num_sections || light_section_idx >= sections.sky_light.len() {
                continue;
            }

            let sky_light =
                sections.sky_light[light_section_idx]
                    .read()
                    .get(x, section_y as usize, z);

            if sky_light > 0 {
                let world_x = (chunk_pos.0.x * 16) + x as i32;
                let world_z = (chunk_pos.0.y * 16) + z as i32;
                let pos = BlockPos(Vector3::new(world_x, y, world_z));

                let block_state =
                    sections.sections[section_idx]
                        .read()
                        .states
                        .get(x, section_y as usize, z);
                let is_empty_shape = !self.has_collision(block_state);

                // Enqueue with all-directional propagation
                sky_engine_horizontal.enqueue_increase(
                    pos,
                    QueueEntry::increase_from_emission(sky_light, is_empty_shape),
                );
            }
        }

        // PHASE 1: Center chunk only horizontal propagation (synchronous, lock-free)
        {
            let (_, sections_for_sky_phase1) = match &**chunk_guard {
                ChunkAccess::Proto(proto_chunk) => (proto_chunk.pos, &proto_chunk.sections),
                ChunkAccess::Full(level_chunk) => (level_chunk.pos, &level_chunk.sections),
            };

            let center_access_sky = CenterOnlyChunkAccess::new(
                chunk_pos,
                sections_for_sky_phase1,
                chunk_min_y,
                self.block_registry.clone(),
                LightType::Sky,
            );

            sky_engine_horizontal.run_center_chunk_updates(&center_access_sky);
        }

        // PHASE 2: Iterative boundary crossing propagation for sky light
        let sky_current_crossings = sky_engine_horizontal.take_boundary_crossings();

        let mut sky_crossings_by_chunk: FxHashMap<ChunkPos, Vec<BoundaryCrossing>> =
            FxHashMap::default();

        for crossing in sky_current_crossings {
            let target_chunk_x = crossing.pos.0.x >> 4;
            let target_chunk_z = crossing.pos.0.z >> 4;
            let target_chunk = ChunkPos(steel_utils::math::Vector2::new(
                target_chunk_x,
                target_chunk_z,
            ));

            sky_crossings_by_chunk
                .entry(target_chunk)
                .or_default()
                .push(crossing);
        }

        let mut sky_consecutive_failed_iterations = 0;

        while !sky_crossings_by_chunk.is_empty() {
            let mut sky_next_crossings_by_chunk: FxHashMap<ChunkPos, Vec<BoundaryCrossing>> =
                FxHashMap::default();
            let mut sky_locked_any_this_iteration = false;

            let mut sky_sorted_chunks: Vec<_> = sky_crossings_by_chunk.into_iter().collect();
            sky_sorted_chunks.sort_by_key(|(chunk_pos, _)| (chunk_pos.0.x, chunk_pos.0.y));

            for (target_chunk, chunk_crossings) in sky_sorted_chunks {
                if target_chunk == chunk_pos {
                    continue;
                }

                let chunk_holder = cache.get(target_chunk.0.x, target_chunk.0.y);
                let chunk_lock_opt = chunk_holder.try_chunk(ChunkStatus::InitializeLight);

                let Some(chunk_lock) = chunk_lock_opt else {
                    sky_next_crossings_by_chunk
                        .entry(target_chunk)
                        .or_default()
                        .extend(chunk_crossings);
                    continue;
                };

                sky_locked_any_this_iteration = true;

                if let Some(chunk_arc) = chunk_lock.as_ref() {
                    let sections = match chunk_arc.as_ref() {
                        ChunkAccess::Proto(proto) => &proto.sections,
                        ChunkAccess::Full(full) => &full.sections,
                    };

                    let mut neighbor_sky_engine = LightEngine::new();

                    for crossing in chunk_crossings {
                        let rel_x = (crossing.pos.0.x & 15) as usize;
                        let rel_y = (crossing.pos.0.y - chunk_min_y) as usize;
                        let rel_z = (crossing.pos.0.z & 15) as usize;

                        let section_idx = rel_y / 16;
                        let section_y = rel_y % 16;
                        let light_section_idx = section_idx + 1;

                        if light_section_idx < sections.sky_light.len() {
                            let current_light = sections.sky_light[light_section_idx]
                                .read()
                                .get(rel_x, section_y, rel_z);
                            let new_light = crossing.entry.level();

                            if new_light > current_light {
                                sections.sky_light[light_section_idx]
                                    .write()
                                    .set(rel_x, section_y, rel_z, new_light);
                                chunk_holder.mark_light_storage_section_changed(
                                    light_section_idx as u32,
                                    true,
                                );

                                neighbor_sky_engine.enqueue_increase(crossing.pos, crossing.entry);
                            }
                        }
                    }

                    if neighbor_sky_engine.has_work() {
                        let neighbor_access_sky = CenterOnlyChunkAccess::new(
                            target_chunk,
                            sections,
                            chunk_min_y,
                            self.block_registry.clone(),
                            LightType::Sky,
                        );

                        neighbor_sky_engine.run_center_chunk_updates(&neighbor_access_sky);

                        let new_crossings = neighbor_sky_engine.take_boundary_crossings();
                        for crossing in new_crossings {
                            let target_chunk_x = crossing.pos.0.x >> 4;
                            let target_chunk_z = crossing.pos.0.z >> 4;
                            let target_chunk = ChunkPos(steel_utils::math::Vector2::new(
                                target_chunk_x,
                                target_chunk_z,
                            ));

                            sky_next_crossings_by_chunk
                                .entry(target_chunk)
                                .or_default()
                                .push(crossing);
                        }
                    }
                }
            }

            if sky_locked_any_this_iteration {
                sky_consecutive_failed_iterations = 0;
            } else if !sky_next_crossings_by_chunk.is_empty() {
                sky_consecutive_failed_iterations += 1;
            }

            if sky_consecutive_failed_iterations >= 10 && !sky_next_crossings_by_chunk.is_empty() {
                tokio::task::yield_now().await;
                sky_consecutive_failed_iterations = 0;
            }

            sky_crossings_by_chunk = sky_next_crossings_by_chunk;
        }

        // Mark all sky light sections in center chunk as changed
        for section_idx in 0..num_light_sections {
            center_holder.mark_light_storage_section_changed(section_idx as u32, true);
        }

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
                LightType::Block,
            );

            engine.run_center_chunk_updates(&center_access);
        }

        // Mark all light sections in center chunk as changed
        for section_idx in 0..num_light_sections {
            center_holder.mark_light_storage_section_changed(section_idx as u32, false);
        }

        // PHASE 2: Iterative boundary crossing propagation
        let current_crossings = engine.take_boundary_crossings();

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
            let mut locked_any_this_iteration = false;

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

                // Successfully locked a chunk this iteration
                locked_any_this_iteration = true;

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
                            LightType::Block,
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

        log::trace!(
            "Chunk {chunk_pos:?} lighting completed in {}µs",
            overall_start.elapsed().as_micros()
        );

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
}
