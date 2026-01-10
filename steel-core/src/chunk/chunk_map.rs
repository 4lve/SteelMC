use std::{
    io, mem,
    sync::{Arc, Weak, atomic::Ordering},
    time::Duration,
};

use rayon::{
    ThreadPool, ThreadPoolBuilder,
    iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator},
};
use rustc_hash::FxBuildHasher;
use steel_protocol::packets::game::{
    BlockChange, CBlockUpdate, CSectionBlocksUpdate, CSetChunkCenter,
};
use steel_registry::{REGISTRY, dimension_type::DimensionTypeRef, vanilla_blocks};
use steel_utils::{BlockPos, ChunkPos, SectionPos, locks::SyncMutex};
use tokio::{runtime::Runtime, time::Instant};
use tokio_util::task::TaskTracker;

use crate::chunk::chunk_access::ChunkAccess;
use crate::chunk::chunk_holder::ChunkHolder;
use crate::chunk::chunk_ticket_manager::{
    ChunkTicketManager, LevelChange, MAX_VIEW_DISTANCE, is_full,
};
use crate::chunk::player_chunk_view::PlayerChunkView;
use crate::chunk::world_gen_context::ChunkGeneratorType;
use crate::chunk::{
    chunk_access::ChunkStatus, chunk_generation_task::ChunkGenerationTask,
    flat_chunk_generator::FlatChunkGenerator, world_gen_context::WorldGenContext,
};
use crate::chunk_saver::RegionManager;
use crate::config::STEEL_CONFIG;
use crate::player::Player;
use crate::world::World;

#[allow(dead_code)]
const PROCESS_CHANGES_WARN_THRESHOLD: usize = 1_000;
#[allow(dead_code)]
const PROCESS_CHANGES_WARN_MIN_DURATION: Duration = Duration::from_micros(500);
const SLOW_TASK_WARN_THRESHOLD: Duration = Duration::from_micros(250);
/// A map of chunks managing their state, loading, and generation.
pub struct ChunkMap {
    /// Map of active chunks.
    pub chunks: scc::HashMap<ChunkPos, Arc<ChunkHolder>, FxBuildHasher>,
    /// Map of chunks currently being unloaded.
    pub unloading_chunks: scc::HashMap<ChunkPos, Arc<ChunkHolder>, FxBuildHasher>,
    /// Queue of pending generation tasks.
    pub pending_generation_tasks: SyncMutex<Vec<Arc<ChunkGenerationTask>>>,
    /// Tracker for background generation tasks.
    pub task_tracker: TaskTracker,
    /// Manager for chunk distances and tickets.
    pub chunk_tickets: SyncMutex<ChunkTicketManager>,
    /// The world generation context.
    pub world_gen_context: Arc<WorldGenContext>,
    /// The thread pool to use for generation.
    pub thread_pool: Arc<ThreadPool>,
    /// The runtime to use for chunk tasks.
    pub chunk_runtime: Arc<Runtime>,
    /// Manager for chunk saving and loading.
    pub region_manager: Arc<RegionManager>,
    /// Chunk holders with pending block changes to broadcast.
    pub chunks_to_broadcast: SyncMutex<Vec<Arc<ChunkHolder>>>,
}

impl ChunkMap {
    /// Creates a new chunk map.
    #[must_use]
    #[allow(clippy::missing_panics_doc, clippy::unwrap_used)]
    pub fn new(
        chunk_runtime: Arc<Runtime>,
        world: Weak<World>,
        dimension: &DimensionTypeRef,
    ) -> Self {
        let generator = Arc::new(ChunkGeneratorType::Flat(FlatChunkGenerator::new(
            REGISTRY
                .blocks
                .get_default_state_id(vanilla_blocks::BEDROCK), // Bedrock
            REGISTRY.blocks.get_default_state_id(vanilla_blocks::DIRT), // Dirt
            REGISTRY
                .blocks
                .get_default_state_id(vanilla_blocks::GRASS_BLOCK), // Grass Block
        )));

        Self {
            chunks: scc::HashMap::with_capacity_and_hasher(1000, FxBuildHasher),
            unloading_chunks: scc::HashMap::with_capacity_and_hasher(1000, FxBuildHasher),
            pending_generation_tasks: SyncMutex::new(Vec::new()),
            task_tracker: TaskTracker::new(),
            chunk_tickets: SyncMutex::new(ChunkTicketManager::new()),
            world_gen_context: Arc::new(WorldGenContext::new(generator, world)),
            thread_pool: Arc::new(ThreadPoolBuilder::new().build().unwrap()),
            chunk_runtime,
            region_manager: Arc::new(RegionManager::new(format!("world/{}", dimension.key.path))),
            chunks_to_broadcast: SyncMutex::new(Vec::new()),
        }
    }

    /// Returns a chunk if it's fully loaded
    #[allow(clippy::missing_panics_doc)]
    pub fn get_full_chunk(&self, pos: &ChunkPos) -> Option<Arc<ChunkAccess>> {
        let chunk_holder = self.chunks.get_sync(pos)?;
        chunk_holder
            .try_chunk(ChunkStatus::Full)
            .map(|guard| guard.as_ref().expect("Not empty by this stage").clone())
    }

    /// Records a block change at the given position.
    /// This marks the chunk as having pending changes to broadcast.
    pub fn block_changed(&self, pos: &BlockPos) {
        let chunk_pos = ChunkPos::new(
            SectionPos::block_to_section_coord(pos.0.x),
            SectionPos::block_to_section_coord(pos.0.z),
        );

        if let Some(holder) = self.chunks.read_sync(&chunk_pos, |_, h| h.clone())
            && holder.block_changed(pos)
        {
            // First change for this chunk - add to broadcast list
            self.chunks_to_broadcast.lock().push(holder);
        }
    }

    /// Broadcasts all pending block changes to nearby players.
    ///
    /// # Panics
    /// Panics if a section has exactly one change (should never happen).
    pub fn broadcast_changed_chunks(&self) {
        let holders = {
            let mut guard = self.chunks_to_broadcast.lock();
            if guard.is_empty() {
                return;
            }
            mem::take(&mut *guard)
        };

        let world = self.world_gen_context.world();

        for holder in holders {
            let chunk_pos = holder.get_pos();
            let min_y = holder.min_y();

            // Take all pending changes from this chunk holder
            let changes_by_section = holder.take_changed_blocks();

            if changes_by_section.is_empty() {
                continue;
            }

            // Get players tracking this chunk
            let tracking_players = world.player_area_map.get_tracking_players(chunk_pos);
            if tracking_players.is_empty() {
                continue;
            }

            // For each section with changes, send appropriate packet
            for (section_index, changed_positions) in changes_by_section {
                let section_y = min_y / 16 + section_index as i32;
                let section_pos = SectionPos::new(chunk_pos.0.x, section_y, chunk_pos.0.y);

                if changed_positions.len() == 1 {
                    // Single block change - use CBlockUpdate
                    let packed = *changed_positions.iter().next().expect("len == 1");
                    let block_pos = section_pos.relative_to_block_pos(packed);
                    let block_state = world.get_block_state(&block_pos);

                    let update_packet = CBlockUpdate {
                        pos: block_pos,
                        block_state,
                    };

                    for uuid in &tracking_players {
                        if let Some(player) = world.players.read_sync(uuid, |_, p| p.clone()) {
                            player.connection.send_packet(update_packet.clone());
                        }
                    }
                } else {
                    // Multiple block changes - use CSectionBlocksUpdate
                    let changes: Vec<BlockChange> = changed_positions
                        .iter()
                        .map(|&packed| {
                            let block_pos = section_pos.relative_to_block_pos(packed);
                            let block_state = world.get_block_state(&block_pos);
                            BlockChange {
                                pos: block_pos,
                                block_state,
                            }
                        })
                        .collect();

                    let packet = CSectionBlocksUpdate {
                        section_pos,
                        changes,
                    };

                    for uuid in &tracking_players {
                        if let Some(player) = world.players.read_sync(uuid, |_, p| p.clone()) {
                            player.connection.send_packet(packet.clone());
                        }
                    }
                }
            }
        }
    }

    /// Schedules a new generation task.
    #[inline]
    pub(crate) fn schedule_generation_task_b(
        self: &Arc<Self>,
        target_status: ChunkStatus,
        pos: ChunkPos,
    ) -> Arc<ChunkGenerationTask> {
        let start = Instant::now();
        let task = Arc::new(ChunkGenerationTask::new(
            pos,
            target_status,
            self.clone(),
            self.thread_pool.clone(),
        ));
        if start.elapsed() >= Duration::from_millis(1) {
            log::warn!("schedule_generation_task_b took: {:?}", start.elapsed());
        }
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
        let tasks = pending.drain(..).collect::<Vec<_>>();
        tasks.into_par_iter().for_each(|task| {
            self.task_tracker
                .spawn_on(async move { task.run().await }, self.chunk_runtime.handle());
        });
    }

    /// Updates scheduling for a chunk based on its new level.
    /// Returns the chunk holder if it is active.
    #[inline]
    #[allow(clippy::missing_panics_doc, clippy::unwrap_used)]
    pub fn update_chunk_level(
        self: &Arc<Self>,
        pos: &ChunkPos,
        new_level: Option<u8>,
    ) -> Option<Arc<ChunkHolder>> {
        // Recover from unloading if possible, else create new holder.
        let chunk_holder =
            if let Some(holder) = self.chunks.read_sync(pos, |_, holder| holder.clone()) {
                holder
            } else {
                new_level?;

                if let Some(entry) = self.unloading_chunks.remove_sync(pos) {
                    let _ = self.chunks.insert_sync(*pos, entry.1.clone());
                    entry.1
                } else {
                    let holder = Arc::new(ChunkHolder::new(
                        *pos,
                        new_level.unwrap(),
                        self.world_gen_context.min_y(),
                        self.world_gen_context.height(),
                    ));
                    let _ = self.chunks.insert_sync(*pos, holder.clone());
                    holder
                }
            };

        if let Some(level) = new_level {
            let old = chunk_holder.ticket_level.swap(level, Ordering::Relaxed);
            if old != level {
                chunk_holder.update_highest_allowed_status(level);
            }
            Some(chunk_holder)
        } else {
            //log::info!("Unloading chunk at {pos:?}");
            chunk_holder.cancel_generation_task();

            // Check for two cause we are also holding a reference to the chunk
            if let Some((_, holder)) = self
                .chunks
                .remove_if_sync(pos, |chunk| Arc::strong_count(chunk) == 2)
            {
                let _ = self.unloading_chunks.insert_sync(*pos, holder.clone());
            } else {
                chunk_holder.ticket_level.store(u8::MAX, Ordering::Relaxed);
                chunk_holder.update_highest_allowed_status(u8::MAX);
            }
            None
        }
    }

    /// Processes chunk updates.
    pub fn tick_b(self: &Arc<Self>, tick_count: u64) {
        let start = Instant::now();

        {
            let mut ct = self.chunk_tickets.lock();

            let updates_start = Instant::now();
            // Only process chunks that actually changed
            let changes: Vec<LevelChange> = ct.run_all_updates().to_vec();
            let updates_elapsed = updates_start.elapsed();

            let holder_creation_start = Instant::now();
            let holders_to_schedule: Vec<_> = changes
                .iter()
                .filter_map(|change| {
                    self.update_chunk_level(&change.pos, change.new_level)
                        .map(|holder| (holder, change.new_level))
                })
                .collect();
            let holder_creation_elapsed = holder_creation_start.elapsed();

            let schedule_start = Instant::now();
            holders_to_schedule.par_iter().for_each(|(holder, level)| {
                if let Some(level) = level
                    && is_full(*level)
                {
                    holder.schedule_chunk_generation_task_b(ChunkStatus::Full, self);
                }
            });
            let schedule_elapsed = schedule_start.elapsed();

            if updates_elapsed >= SLOW_TASK_WARN_THRESHOLD {
                log::warn!("chunk_tickets run_updates slow: {updates_elapsed:?}");
            }
            if holder_creation_elapsed >= SLOW_TASK_WARN_THRESHOLD {
                log::warn!("holder_creation slow: {holder_creation_elapsed:?}");
            }
            if schedule_elapsed >= SLOW_TASK_WARN_THRESHOLD {
                log::warn!("schedule slow: {schedule_elapsed:?}");
            }
        };

        let start_gen = Instant::now();
        self.run_generation_tasks_b();
        let gen_elapsed = start_gen.elapsed();
        if gen_elapsed >= SLOW_TASK_WARN_THRESHOLD {
            log::warn!("run_generation_tasks_b slow: {gen_elapsed:?}");
        }

        let start_broadcast = Instant::now();
        self.broadcast_changed_chunks();
        let broadcast_elapsed = start_broadcast.elapsed();
        if broadcast_elapsed >= SLOW_TASK_WARN_THRESHOLD {
            log::warn!("broadcast_changed_chunks slow: {broadcast_elapsed:?}");
        }

        let start_unload = Instant::now();
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
                self.unloading_chunks.len()
            );
        }
    }

    /// Saves a chunk to disk.
    ///
    /// This function is currently a placeholder for the actual saving logic.
    #[allow(clippy::missing_panics_doc, clippy::unwrap_used)]
    pub async fn save_chunk(&self, chunk_holder: &Arc<ChunkHolder>, chunk_map: &Arc<ChunkMap>) {
        let chunk = chunk_holder.try_chunk(ChunkStatus::StructureStarts);

        if let Some(chunk) = chunk {
            let status = chunk_holder
                .persisted_status()
                .expect("The check above confirmed it exists");

            let result = chunk_map.region_manager.save_chunk(&chunk, status).await;

            match result {
                Ok(_) => {
                    let res = chunk_map
                        .unloading_chunks
                        .remove_async(&chunk_holder.get_pos())
                        .await;
                    if res.is_some() {
                        if let Err(e) = chunk_map
                            .region_manager
                            .release_chunk(chunk_holder.get_pos())
                            .await
                        {
                            log::error!("Error releasing chunk: {e}");
                        }
                    } else {
                        // Chunk was recovered
                    }
                }
                Err(e) => {
                    log::error!("Error saving chunk: {e}");
                }
            }
        } else {
            // Chunk was at Empty stage so no need to save it
            let _ = chunk_map
                .unloading_chunks
                .remove_async(&chunk_holder.get_pos())
                .await;
        }
    }

    /// Processes chunks that are pending unload.
    ///
    /// This method iterates over the chunks in the `unloading_chunks` map.
    /// If a chunk is only held by the map (strong count is 1), it is removed
    /// and a background task is spawned to save it.
    pub fn process_unloads(self: &Arc<Self>) {
        self.unloading_chunks.iter_sync(|_pos, holder| {
            // If the strong count is 1, it means only this map holds a reference to the chunk.
            // We can safely unload it.
            if Arc::strong_count(holder) == 1 {
                let holder_clone = holder.clone();
                let map_clone = self.clone();

                self.task_tracker.spawn(async move {
                    map_clone.save_chunk(&holder_clone, &map_clone).await;
                });
            }
            true
        });
    }

    /// Updates the player's status in the chunk map.
    pub fn update_player_status(&self, player: &Player) {
        let current_chunk_pos = *player.last_chunk_pos.lock();
        let view_distance = STEEL_CONFIG.view_distance;

        let new_view = PlayerChunkView::new(current_chunk_pos, view_distance);
        let mut last_view_guard = player.last_tracking_view.lock();

        if last_view_guard.as_ref() != Some(&new_view) {
            let mut chunk_tickets = self.chunk_tickets.lock();

            let connection = &player.connection;
            let world = self.world_gen_context.world();

            if let Some(last_view) = last_view_guard.as_ref() {
                if last_view.center != new_view.center
                    || last_view.view_distance != new_view.view_distance
                {
                    chunk_tickets.remove_ticket(
                        last_view.center,
                        MAX_VIEW_DISTANCE.saturating_sub(last_view.view_distance),
                    );
                    chunk_tickets.add_ticket(
                        new_view.center,
                        MAX_VIEW_DISTANCE.saturating_sub(new_view.view_distance),
                    );

                    connection.send_packet(CSetChunkCenter {
                        x: new_view.center.0.x,
                        y: new_view.center.0.y,
                    });
                }

                // Track chunks for PlayerAreaMap update
                let mut added_chunks = Vec::new();
                let mut removed_chunks = Vec::new();

                // We lock here to ensure we have unique access for the duration of the diff
                let mut chunk_sender = player.chunk_sender.lock();
                PlayerChunkView::difference(
                    last_view,
                    &new_view,
                    |pos, ctx: &mut (&mut _, &mut Vec<_>, &mut Vec<_>)| {
                        ctx.0.mark_chunk_pending_to_send(pos);
                        ctx.1.push(pos);
                    },
                    |pos, ctx: &mut (&mut _, &mut Vec<_>, &mut Vec<_>)| {
                        ctx.0.drop_chunk(connection, pos);
                        ctx.2.push(pos);
                    },
                    &mut (&mut chunk_sender, &mut added_chunks, &mut removed_chunks),
                );
                drop(chunk_sender);

                // Update the player area map with the diff
                world.player_area_map.on_player_view_change(
                    player.gameprofile.id,
                    &added_chunks,
                    &removed_chunks,
                );
            } else {
                chunk_tickets.add_ticket(
                    new_view.center,
                    MAX_VIEW_DISTANCE.saturating_sub(new_view.view_distance),
                );

                let mut chunk_sender = player.chunk_sender.lock();
                new_view.for_each(|pos| {
                    chunk_sender.mark_chunk_pending_to_send(pos);
                });
                drop(chunk_sender);

                // First time - add all chunks in view to player area map
                world.player_area_map.on_player_join(player, &new_view);
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
            let mut chunk_tickets = self.chunk_tickets.lock();
            chunk_tickets.remove_ticket(
                last_view.center,
                MAX_VIEW_DISTANCE.saturating_sub(last_view.view_distance),
            );
        }
    }

    /// Saves all dirty chunks to disk.
    ///
    /// This method should be called during graceful shutdown to ensure all
    /// modified chunks are persisted. It saves:
    /// 1. All dirty chunks in the active `chunks` map
    /// 2. All chunks pending unload in the `unloading_chunks` map
    /// 3. Closes all region file handles (flushing headers)
    ///
    /// Returns the number of chunks saved.
    pub async fn save_all_chunks(self: &Arc<Self>) -> io::Result<usize> {
        let mut saved_count = 0;

        // Collect all chunks from both maps
        let all_chunks: Vec<Arc<ChunkHolder>> = {
            let mut chunks = Vec::new();
            self.chunks.iter_sync(|_, holder| {
                chunks.push(holder.clone());
                true
            });
            self.unloading_chunks.iter_sync(|_, holder| {
                chunks.push(holder.clone());
                true
            });
            chunks
        };

        log::info!("Saving {} chunks...", all_chunks.len());

        // Save all chunks that have data
        for holder in &all_chunks {
            if let Some(chunk) = holder.try_chunk(ChunkStatus::StructureStarts)
                && let Some(status) = holder.persisted_status()
            {
                match self.region_manager.save_chunk(&chunk, status).await {
                    Ok(true) => saved_count += 1,
                    Ok(false) => {} // Not dirty
                    Err(e) => {
                        log::error!("Failed to save chunk at {:?}: {e}", holder.get_pos());
                    }
                }
            }
        }

        // Close all region files (flushes headers and releases file handles)
        if let Err(e) = self.region_manager.close_all().await {
            log::error!("Failed to close region files: {e}");
        }

        log::info!(
            "Saved {saved_count} dirty chunks (checked {} total)",
            all_chunks.len()
        );

        Ok(saved_count)
    }
}
