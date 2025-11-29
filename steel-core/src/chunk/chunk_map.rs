use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use parking_lot::Mutex as ParkingMutex;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rayon::{ThreadPool, ThreadPoolBuilder};
use rustc_hash::{FxBuildHasher, FxHashMap};

use steel_protocol::packets::game::CSetChunkCenter;
use steel_registry::blocks::BlockRegistry;
use steel_registry::vanilla_blocks;
use steel_utils::ChunkPos;
use tokio::runtime::Runtime;
use tokio_util::task::TaskTracker;

use crate::chunk::chunk_holder::ChunkHolder;
use crate::chunk::chunk_level::ChunkLevel;
use crate::chunk::chunk_tracking_view::ChunkTrackingView;
use crate::chunk::{
    chunk_access::ChunkStatus, chunk_generation_task::ChunkGenerationTask,
    chunk_tracker::MAX_LEVEL, distance_manager::DistanceManager,
    flat_chunk_generator::FlatChunkGenerator, world_gen_context::WorldGenContext,
};
use crate::config::STEEL_CONFIG;
use crate::player::Player;

const PROCESS_CHANGES_WARN_THRESHOLD: usize = 1_000;
const PROCESS_CHANGES_WARN_MIN_DURATION: Duration = Duration::from_micros(500);
const SLOW_TASK_WARN_THRESHOLD: Duration = Duration::from_micros(250);
/// A map of chunks managing their state, loading, and generation.
pub struct ChunkMap {
    /// Map of active chunks.
    pub chunks: scc::HashMap<ChunkPos, Arc<ChunkHolder>, FxBuildHasher>,
    /// Map of chunks currently being unloaded.
    pub unloading_chunks: ParkingMutex<FxHashMap<ChunkPos, Arc<ChunkHolder>>>,
    /// Queue of pending generation tasks.
    pub pending_generation_tasks: ParkingMutex<Vec<Arc<ChunkGenerationTask>>>,
    /// Tracker for background generation tasks.
    pub task_tracker: TaskTracker,
    /// Manager for chunk distances and tickets.
    pub distance_manager: ParkingMutex<DistanceManager>,
    /// The world generation context.
    pub world_gen_context: Arc<WorldGenContext>,
    /// The thread pool to use for generation.
    pub thread_pool: Arc<ThreadPool>,
    /// The runtime to use for chunk tasks.
    pub chunk_runtime: Arc<Runtime>,
}

impl ChunkMap {
    /// Creates a new chunk map.
    #[must_use]
    pub fn new(block_registry: &BlockRegistry, chunk_runtime: Arc<Runtime>) -> Self {
        Self {
            chunks: scc::HashMap::with_capacity_and_hasher(1000, FxBuildHasher),
            unloading_chunks: ParkingMutex::new(FxHashMap::with_capacity_and_hasher(
                1000,
                FxBuildHasher,
            )),
            pending_generation_tasks: ParkingMutex::new(Vec::new()),
            task_tracker: TaskTracker::new(),
            distance_manager: ParkingMutex::new(DistanceManager::new()),
            world_gen_context: Arc::new(WorldGenContext {
                generator: Arc::new(FlatChunkGenerator::new(
                    block_registry.get_default_state_id(vanilla_blocks::BEDROCK), // Bedrock
                    block_registry.get_default_state_id(vanilla_blocks::DIRT),    // Dirt
                    block_registry.get_default_state_id(vanilla_blocks::GRASS_BLOCK), // Grass Block
                )),
            }),
            thread_pool: Arc::new(ThreadPoolBuilder::new().build().unwrap()),
            chunk_runtime,
        }
    }

    /// Schedules a new generation task.
    pub(crate) fn schedule_generation_task_b(
        self: &Arc<Self>,
        target_status: ChunkStatus,
        pos: ChunkPos,
    ) -> Arc<ChunkGenerationTask> {
        let task = Arc::new(ChunkGenerationTask::new(
            pos,
            target_status,
            self.clone(),
            self.thread_pool.clone(),
        ));
        self.pending_generation_tasks.lock().push(task.clone());
        task
    }

    /// Runs queued generation tasks.
    pub fn run_generation_tasks_b(&self) {
        let mut pending = self.pending_generation_tasks.lock();
        if pending.is_empty() {
            return;
        }
        //log::info!("Running {} generation tasks", pending.len());
        for task in pending.drain(..) {
            self.task_tracker.spawn_on(
                async move {
                    task.run().await;
                },
                self.chunk_runtime.handle(),
            );
        }
    }

    /// Updates scheduling for a chunk based on its new level.
    /// Returns the chunk holder if it is active.
    #[inline]
    pub fn update_chunk_level(
        self: &Arc<Self>,
        pos: &ChunkPos,
        new_level: u8,
    ) -> Option<Arc<ChunkHolder>> {
        // Recover from unloading if possible, else create new holder.
        let chunk_holder = if let Some(holder) = self.chunks.get_sync(pos) {
            holder.get().clone()
        } else {
            if new_level >= MAX_LEVEL {
                return None;
            }

            if let Some(entry) = self.remove_from_unloading(pos) {
                entry.clear_pending_unload();
                let _ = self.chunks.insert_sync(*pos, entry.clone());
                entry
            } else {
                let holder = Arc::new(ChunkHolder::new(*pos, new_level));
                let _ = self.chunks.insert_sync(*pos, holder.clone());
                holder
            }
        };

        if new_level >= MAX_LEVEL {
            //log::info!("Unloading chunk at {pos:?}");
            if chunk_holder.mark_pending_unload() {
                chunk_holder.force_fail();
            }

            // Check for two cause we are also holding a reference to the chunk
            if let Some((_, holder)) = self
                .chunks
                .remove_if_sync(pos, |chunk| Arc::strong_count(chunk) == 2)
            {
                self.insert_into_unloading(*pos, holder);
            } else {
                chunk_holder
                    .ticket_level
                    .store(new_level, Ordering::Relaxed);
                chunk_holder.update_highest_allowed_status(new_level);
            }
            drop(chunk_holder);
            None
        } else {
            chunk_holder.clear_pending_unload();

            let current_level = chunk_holder.ticket_level.load(Ordering::Relaxed);
            if current_level == new_level {
                return Some(chunk_holder);
            }

            chunk_holder
                .ticket_level
                .store(new_level, Ordering::Relaxed);
            chunk_holder.update_highest_allowed_status(new_level);
            Some(chunk_holder)
        }
    }

    fn remove_from_unloading(&self, pos: &ChunkPos) -> Option<Arc<ChunkHolder>> {
        if let Some(mut guard) = self.unloading_chunks.try_lock() {
            guard.remove(pos)
        } else {
            let mut guard = self.unloading_chunks.lock();
            guard.remove(pos)
        }
    }

    fn insert_into_unloading(&self, pos: ChunkPos, holder: Arc<ChunkHolder>) {
        if let Some(mut guard) = self.unloading_chunks.try_lock() {
            let _ = guard.insert(pos, holder);
        } else {
            let mut guard = self.unloading_chunks.lock();
            let _ = guard.insert(pos, holder);
        }
    }

    /// Processes chunk updates.
    pub fn tick_b(self: &Arc<Self>, tick_count: u64) {
        let start = tokio::time::Instant::now();

        let (changes, purge_elapsed, updates_elapsed) = {
            let mut dm = self.distance_manager.lock();

            let purge_start = tokio::time::Instant::now();
            dm.purge_tickets(tick_count);
            let purge_elapsed = purge_start.elapsed();

            let updates_start = tokio::time::Instant::now();
            let changes = dm.run_updates();
            let updates_elapsed = updates_start.elapsed();

            (changes, purge_elapsed, updates_elapsed)
        };

        if purge_elapsed >= SLOW_TASK_WARN_THRESHOLD {
            log::warn!("distance_manager purge_tickets slow: {purge_elapsed:?}");
        }
        if updates_elapsed >= SLOW_TASK_WARN_THRESHOLD {
            log::warn!("distance_manager run_updates slow: {updates_elapsed:?}");
        }

        let deduped: FxHashMap<_, _> = changes
            .iter()
            .map(|(pos, _, new_level)| (*pos, *new_level))
            .collect();        

        let start_process_changes = tokio::time::Instant::now();
        let deduped_len = deduped.len();

        let updates_to_schedule: Vec<(_, _)> = deduped
            .into_iter()
            .filter_map(|(pos, new_level)| {
                self.update_chunk_level(&pos, new_level)
                    .map(|holder| (holder, new_level))
            })
            .collect();

        let process_elapsed = start_process_changes.elapsed();
        if !updates_to_schedule.is_empty()
            && (updates_to_schedule.len() >= PROCESS_CHANGES_WARN_THRESHOLD
                || process_elapsed >= PROCESS_CHANGES_WARN_MIN_DURATION)
        {
            let per_change =
                process_elapsed.as_secs_f64() * 1_000_000.0 / updates_to_schedule.len() as f64;
            // Show unique (deduped) vs scheduled counts. Avoid unsigned underflow from incorrect subtraction.
            log::warn!(
                "process changes: {:?} ({} unique, {} scheduled, {:.2}µs/change)",
                process_elapsed,
                deduped_len,
                updates_to_schedule.len(),
                per_change
            );
        }

        let schedule_start = tokio::time::Instant::now();
        let self_clone = self.clone();
        let update_len = updates_to_schedule.len();

        updates_to_schedule
            .par_iter()
            .for_each(|(chunk_holder, new_level)| {
                let target_status = ChunkLevel::generation_status(*new_level);

                if let Some(status) = target_status
                    && status == ChunkStatus::Full
                {
                    let chunk_holder_clone = chunk_holder.clone();
                    let map_clone = self_clone.clone();
                    chunk_holder_clone.schedule_chunk_generation_task_b(status, map_clone);
                }
            });

        let schedule_elapsed = schedule_start.elapsed();
        if schedule_elapsed >= SLOW_TASK_WARN_THRESHOLD {
            log::warn!("tick_b schedule loop took: {schedule_elapsed:?} ({update_len} updates)");
        }

        let start_gen = tokio::time::Instant::now();
        self.run_generation_tasks_b();
        let gen_elapsed = start_gen.elapsed();
        if gen_elapsed >= SLOW_TASK_WARN_THRESHOLD {
            log::warn!("run_generation_tasks_b slow: {gen_elapsed:?}");
        }

        let start_unload = tokio::time::Instant::now();
        self.process_unloads();
        let unload_elapsed = start_unload.elapsed();
        if unload_elapsed >= SLOW_TASK_WARN_THRESHOLD {
            log::warn!("process_unloads slow: {unload_elapsed:?}");
        }

        let tick_elapsed = start.elapsed();
        if tick_elapsed >= SLOW_TASK_WARN_THRESHOLD {
            log::warn!("Tick_b slow: total {tick_elapsed:?}");
        }

        if tick_count.is_multiple_of(100) {
            log::debug!(
                "Chunk map entries: {}, unloading chunks: {}",
                self.chunks.len(),
                self.unloading_chunks.lock().len()
            );
        }
    }

    /// Saves a chunk to disk.
    ///
    /// This function is currently a placeholder for the actual saving logic.
    pub async fn save_chunk(&self, chunk_holder: &Arc<ChunkHolder>) {
        let _pos = chunk_holder.get_pos();
        // Access the chunk to ensure it's loaded and ready for saving
        // We use ChunkStatus::StructureReferences as the minimum requirement, effectively checking if any data exists.
        let _saved = chunk_holder.try_chunk(ChunkStatus::StructureReferences);
        //TODO: Save the chunk to disk
    }

    /// Processes chunks that are pending unload.
    ///
    /// This method iterates over the chunks in the `unloading_chunks` map.
    /// If a chunk is only held by the map (strong count is 1), it is removed
    /// and a background task is spawned to save it.
    pub fn process_unloads(self: &Arc<Self>) {
        self.unloading_chunks.lock().retain(|_, holder| {
            // If the strong count is 1, it means only this map holds a reference to the chunk.
            // We can safely unload it.
            if Arc::strong_count(&*holder) == 1 {
                let holder_clone = holder.clone();
                let map_clone = self.clone();

                self.task_tracker.spawn(async move {
                    map_clone.save_chunk(&holder_clone).await;
                });
                // Remove from unloading_chunks.
                return false;
            }
            true
        });
    }

    /// Updates the player's status in the chunk map.
    pub fn update_player_status(&self, player: &Player) {
        let current_chunk_pos = *player.last_chunk_pos.lock();
        let view_distance = STEEL_CONFIG.view_distance;

        let new_view = ChunkTrackingView::new(current_chunk_pos, i32::from(view_distance));
        let mut last_view_guard = player.last_tracking_view.lock();

        if last_view_guard.as_ref() != Some(&new_view) {
            let mut distance_manager = self.distance_manager.lock();

            let connection = &player.connection;

            if let Some(last_view) = last_view_guard.as_ref() {
                if last_view.center != new_view.center
                    || last_view.view_distance != new_view.view_distance
                {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    distance_manager.remove_player(last_view.center, last_view.view_distance as u8);
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    distance_manager.add_player(new_view.center, new_view.view_distance as u8);

                    connection.send_packet(CSetChunkCenter {
                        x: new_view.center.0.x,
                        y: new_view.center.0.y,
                    });
                }

                // We lock here to ensure we have unique access for the duration of the diff
                let mut chunk_sender = player.chunk_sender.lock();
                ChunkTrackingView::difference(
                    last_view,
                    &new_view,
                    |pos, chunk_sender| {
                        chunk_sender.mark_chunk_pending_to_send(pos);
                    },
                    |pos, chunk_sender| {
                        chunk_sender.drop_chunk(connection, pos);
                    },
                    &mut chunk_sender,
                );
            } else {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                distance_manager.add_player(new_view.center, new_view.view_distance as u8);

                let mut chunk_sender = player.chunk_sender.lock();
                new_view.for_each(|pos| {
                    chunk_sender.mark_chunk_pending_to_send(pos);
                });
            }

            *last_view_guard = Some(new_view);
        }
    }

    /// Removes a player from the chunk map.
    pub fn remove_player(&self, player: &Player) {
        // Okay to lock sync lock here cause it has low contention
        let mut last_view_guard = player.last_tracking_view.lock();
        if let Some(last_view) = last_view_guard.take() {
            drop(last_view_guard);
            let mut distance_manager = self.distance_manager.lock();
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            distance_manager.remove_player(last_view.center, last_view.view_distance as u8);
        }
    }
}
