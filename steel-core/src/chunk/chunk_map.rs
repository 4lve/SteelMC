use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use parking_lot::Mutex as ParkingMutex;
use steel_protocol::packets::game::CSetChunkCenter;
use steel_utils::ChunkPos;
use tokio::sync::Mutex;
use tokio_util::task::TaskTracker;

use crate::chunk::chunk_holder::ChunkHolder;
use crate::chunk::chunk_tracking_view::ChunkTrackingView;
use crate::chunk::{
    chunk_access::ChunkStatus, chunk_generation_task::ChunkGenerationTask,
    chunk_generator::SimpleChunkGenerator, chunk_pyramid::GENERATION_PYRAMID,
    chunk_tracker::MAX_LEVEL, distance_manager::DistanceManager,
    world_gen_context::WorldGenContext,
};
use crate::config::STEEL_CONFIG;
use crate::player::Player;

const PROCESS_CHANGES_WARN_THRESHOLD: usize = 1_000;
const PROCESS_CHANGES_WARN_MIN_DURATION: Duration = Duration::from_micros(500);

/// A map of chunks managing their state, loading, and generation.
pub struct ChunkMap {
    /// Map of active chunks.
    pub chunks: scc::HashMap<ChunkPos, Arc<ChunkHolder>>,
    /// Map of chunks currently being unloaded.
    pub unloading_chunks: scc::HashMap<ChunkPos, Arc<ChunkHolder>>,
    /// Queue of pending generation tasks.
    pub pending_generation_tasks: Mutex<Vec<Arc<ChunkGenerationTask>>>,
    /// Tracker for background generation tasks.
    pub task_tracker: TaskTracker,
    /// Manager for chunk distances and tickets.
    pub distance_manager: ParkingMutex<DistanceManager>,
    /// The world generation context.
    pub world_gen_context: Arc<WorldGenContext>,
}

impl Default for ChunkMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkMap {
    /// Creates a new chunk map.
    #[must_use]
    pub fn new() -> Self {
        Self {
            chunks: scc::HashMap::new(),
            unloading_chunks: scc::HashMap::new(),
            pending_generation_tasks: Mutex::new(Vec::new()),
            task_tracker: TaskTracker::new(),
            distance_manager: ParkingMutex::new(DistanceManager::new()),
            world_gen_context: Arc::new(WorldGenContext {
                generator: Arc::new(SimpleChunkGenerator),
            }),
        }
    }

    /// Schedules a new generation task.
    pub(crate) async fn schedule_generation_task(
        self: &Arc<Self>,
        target_status: ChunkStatus,
        pos: ChunkPos,
    ) -> Arc<ChunkGenerationTask> {
        let task = Arc::new(ChunkGenerationTask::new(pos, target_status, self.clone()).await);
        self.pending_generation_tasks
            .lock()
            .await
            .push(task.clone());
        task
    }

    /// Runs queued generation tasks.
    pub fn run_generation_tasks_b(&self) {
        let mut pending = self.pending_generation_tasks.blocking_lock();
        if pending.is_empty() {
            return;
        }
        //log::info!("Running {} generation tasks", pending.len());
        for task in pending.drain(..) {
            self.task_tracker.spawn(async move {
                task.run().await;
            });
        }
    }

    /// Updates scheduling for a chunk based on its new level.
    /// Returns the chunk holder if it is active.
    pub fn update_chunk_level_b(
        self: &Arc<Self>,
        pos: ChunkPos,
        new_level: u8,
    ) -> Option<Arc<ChunkHolder>> {
        // Recover from unloading if possible, else create new holder.
        let chunk_holder = if let Some(entry) = self.unloading_chunks.remove_sync(&pos) {
            let holder = entry.1;
            let _ = self.chunks.insert_sync(pos, holder.clone());
            holder
        } else if let Some(holder) = self.chunks.get_sync(&pos) {
            holder.get().clone()
        } else {
            if new_level >= MAX_LEVEL {
                return None;
            }
            let holder = Arc::new(ChunkHolder::new(pos, new_level));
            let _ = self.chunks.insert_sync(pos, holder.clone());
            holder
        };

        *chunk_holder.ticket_level.blocking_lock() = new_level;

        //log::info!("New level: {new_level}");
        if new_level >= MAX_LEVEL {
            //log::info!("Unloading chunk at {pos:?}");
            chunk_holder.cancel_generation_task();
            // Drop the local reference so it doesn't count towards the strong count
            drop(chunk_holder);

            if let Some((_, holder)) = self.chunks.remove_sync(&pos) {
                let _ = self.unloading_chunks.insert_sync(pos, holder);
            }
            None
        } else {
            Some(chunk_holder)
        }
    }

    /// Processes chunk updates.
    pub fn tick_b(self: &Arc<Self>, tick_count: u64) {
        let start = tokio::time::Instant::now();

        let changes = {
            let mut dm = self.distance_manager.lock();
            dm.purge_tickets(tick_count);
            dm.run_updates()
        };

        let start_process_changes = tokio::time::Instant::now();

        let updates_to_schedule: Vec<_> = {
            let mut result = Vec::with_capacity(changes.len());

            for (pos, _, new_level) in changes {
                if let Some(holder) = self.update_chunk_level_b(pos, new_level) {
                    result.push((holder, new_level));
                }
            }

            result
        };

        let process_elapsed = start_process_changes.elapsed();
        if !updates_to_schedule.is_empty()
            && (updates_to_schedule.len() >= PROCESS_CHANGES_WARN_THRESHOLD
                || process_elapsed >= PROCESS_CHANGES_WARN_MIN_DURATION)
        {
            let per_change = process_elapsed.as_secs_f64() * 1_000_000.0
                / updates_to_schedule.len() as f64;
            log::warn!(
                "process changes: {:?} ({} changes, {:.2}µs/change)",
                process_elapsed,
                updates_to_schedule.len(),
                per_change
            );
        }

        let self_clone = self.clone();
        let _ = self.task_tracker.spawn(async move {
            let tasks = updates_to_schedule
                .into_iter()
                .filter_map(|(chunk_holder, new_level)| {
                    let target_status = if new_level >= MAX_LEVEL {
                        None
                    } else if new_level <= 33 {
                        Some(ChunkStatus::Full)
                    } else {
                        let distance = (new_level - 33) as usize;
                        GENERATION_PYRAMID
                            .get_step_to(ChunkStatus::Full)
                            .accumulated_dependencies
                            .get(distance)
                    };

                    target_status.map(|status| {
                        let chunk_holder_clone = chunk_holder.clone();
                        let map_clone = self_clone.clone();
                        async move {
                            chunk_holder_clone
                                .schedule_chunk_generation_task(status, map_clone)
                                .await;
                        }
                    })
                })
                .collect::<Vec<_>>();

            if !tasks.is_empty() {
                join_all(tasks).await;
            }
        });

        let start_gen = tokio::time::Instant::now();
        self.run_generation_tasks_b();
        if start_gen.elapsed().as_millis() > 1 {
            log::warn!("run_generation_tasks_b slow: {:?}", start_gen.elapsed());
        }

        let start_unload = tokio::time::Instant::now();
        self.process_unloads();
        if start_unload.elapsed().as_millis() > 1 {
            log::warn!("process_unloads slow: {:?}", start_unload.elapsed());
        }

        if start.elapsed().as_millis() > 1 {
            log::warn!("Tick_b slow: total {:?}", start.elapsed());
        }

        if tick_count.is_multiple_of(100) {
            log::debug!(
                "Chunk map entries: {}, unloading chunks: {}",
                self.chunks.len(),
                self.unloading_chunks.len()
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
        let saved = chunk_holder.with_chunk(ChunkStatus::StructureReferences, |_chunk| {
            // TODO: Serialize the chunk data here.
            // Since serialization might be CPU intensive, we might want to do it inside this closure
            // or clone the necessary data structure if possible (though deep cloning chunks is expensive).
            // For now, we assume serialization happens here synchronously.
            true
        });

        if saved.is_some() {
            // TODO: Perform the actual disk I/O here (asynchronously).
            // storage.write(pos, serialized_data).await;
            //log::info!("Saved chunk at {:?}", pos);
        } else {
            //log::warn!(
            //    "Skipping save for chunk at {:?}: Chunk not fully loaded",
            //    pos
            //);
        }
    }

    /// Processes chunks that are pending unload.
    ///
    /// This method iterates over the chunks in the `unloading_chunks` map.
    /// If a chunk is only held by the map (strong count is 1), it is removed
    /// and a background task is spawned to save it.
    pub fn process_unloads(self: &Arc<Self>) {
        self.unloading_chunks.retain_sync(|_, holder| {
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
                ChunkTrackingView::difference(
                    last_view,
                    &new_view,
                    |pos| {
                        player.chunk_sender.lock().mark_chunk_pending_to_send(pos);
                    },
                    |pos| {
                        player.chunk_sender.lock().drop_chunk(connection, pos);
                    },
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
    pub async fn remove_player(&self, player: &Player) {
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
