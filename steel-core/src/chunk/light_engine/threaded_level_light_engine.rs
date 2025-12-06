//! Threaded light engine with task queue system.
//!
//! This extends the base `LightEngine` with asynchronous task scheduling and batched execution.
//! Tasks are divided into `PRE_UPDATE` (setup) and `POST_UPDATE` (completion) phases.

use std::sync::Arc;

use parking_lot::{Mutex, RwLock as ParkingRwLock};
use steel_utils::ChunkPos;

use crate::chunk::{chunk_access::ChunkAccess, section::ChunkSection};

use super::base::LightEngine;

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
    /// This method should:
    /// 1. Call `propagateLightSources()` for both block and sky light
    /// 2. Call `run_light_updates()` to propagate all queued light
    /// 3. Mark chunk as `light_correct`
    ///
    /// # Note
    /// This is currently a TEMPORARY stub implementation that directly sets sky light.
    /// The proper implementation will use the queue-based flood-fill algorithm.
    pub fn light_chunk(
        &self,
        chunk: &ParkingRwLock<Option<ChunkAccess>>,
        _light_enabled: bool,
    ) -> Result<(), anyhow::Error> {
        use crate::chunk::{chunk_generator::ChunkGuard, light_storage::LightStorage};
        use steel_utils::BlockStateId;

        // TEMPORARY: Direct sky light initialization
        // TODO: Replace with proper light engine implementation:
        // - propagate_sky_light_sources(chunk_pos)
        // - propagate_block_light_sources(chunk_pos)
        // - run_light_updates()

        let mut chunk_guard = ChunkGuard::new(chunk);
        let sections = match &mut *chunk_guard {
            ChunkAccess::Proto(proto_chunk) => &mut proto_chunk.sections,
            ChunkAccess::Full(level_chunk) => &mut level_chunk.sections,
        };

        let num_sections = sections.sections.len();

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

        Ok(())
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
