//! This module is responsible for sending chunks to the client.
use rustc_hash::FxHashSet;
use std::sync::Arc;

use steel_protocol::packets::game::{
    CChunkBatchFinished, CChunkBatchStart, CForgetLevelChunk, CLevelChunkWithLight, CLightUpdate,
};
use steel_utils::ChunkPos;
use tokio::task::spawn_blocking;

use crate::{
    chunk::{
        chunk_access::{ChunkAccess, ChunkStatus},
        chunk_holder::ChunkHolder,
    },
    player::networking::JavaConnection,
    world::World,
};

/// This struct is responsible for sending chunks to the client.
#[derive(Debug)]
pub struct ChunkSender {
    /// A list of chunks that are waiting to be sent to the client.
    pub pending_chunks: FxHashSet<ChunkPos>,
    /// The number of batches that have been sent to the client but have not been acknowledged yet.
    pub unacknowledged_batches: u16,
    /// The number of chunks that should be sent to the client per tick.
    pub desired_chunks_per_tick: f32,
    /// The number of chunks that can be sent to the client in the current batch.
    pub batch_quota: f32,
    /// The maximum number of unacknowledged batches allowed.
    pub max_unacknowledged_batches: u16,
}

impl ChunkSender {
    /// Marks a chunk as pending to be sent to the client.
    pub fn mark_chunk_pending_to_send(&mut self, pos: ChunkPos) {
        self.pending_chunks.insert(pos);
    }

    /// Drops a chunk from the client's view.
    pub fn drop_chunk(&mut self, connection: &JavaConnection, pos: ChunkPos) {
        if !self.pending_chunks.remove(&pos) && !connection.closed() {
            connection.send_packet(CForgetLevelChunk { pos });
        }
    }

    /// Sends the next batch of chunks to the client.
    ///
    /// # Panics
    /// Panics if a chunk is not at Full status when it should be.
    pub fn send_next_chunks(
        &mut self,
        connection: Arc<JavaConnection>,
        world: &World,
        player_chunk_pos: ChunkPos,
    ) {
        // First, broadcast any light updates for already-sent chunks
        Self::broadcast_light_updates(connection.clone(), world, player_chunk_pos);

        if self.unacknowledged_batches < self.max_unacknowledged_batches {
            let max_batch_size = self.desired_chunks_per_tick.max(1.0);
            self.batch_quota =
                (self.batch_quota + self.desired_chunks_per_tick).min(max_batch_size);

            if self.batch_quota >= 1.0 && !self.pending_chunks.is_empty() {
                let chunks_to_process = self.collect_candidates(world, player_chunk_pos);
                if !chunks_to_process.is_empty() {
                    self.unacknowledged_batches += 1;
                    self.batch_quota -= chunks_to_process.len() as f32;

                    #[allow(clippy::let_underscore_future)]
                    let _ = spawn_blocking(move || {
                        let mut chunks_to_send = Vec::new();
                        for holder in chunks_to_process {
                            if let Some(chunk) = holder.try_chunk(ChunkStatus::Full).map(|chunk| {
                                if let ChunkAccess::Full(chunk) =
                                    chunk.as_ref().expect("Chunk is not loaded").as_ref()
                                {
                                    CLevelChunkWithLight {
                                        pos: holder.get_pos(),
                                        chunk_data: chunk.extract_chunk_data(),
                                        light_data: chunk.extract_light_data(),
                                    }
                                } else {
                                    panic!("Chunk must be at Full status to be sent to the client");
                                }
                            }) {
                                chunks_to_send.push(chunk);
                            }
                        }

                        connection.send_packet(CChunkBatchStart {});
                        let batch_size = chunks_to_send.len();

                        for chunk in chunks_to_send {
                            connection.send_packet(chunk);
                        }

                        connection.send_packet(CChunkBatchFinished {
                            batch_size: batch_size as i32,
                        });
                    });
                }
            }
        }
    }

    fn collect_candidates(
        &mut self,
        world: &World,
        player_chunk_pos: ChunkPos,
    ) -> Vec<Arc<ChunkHolder>> {
        let max_batch_size = self.batch_quota.floor() as usize;
        let mut candidates: Vec<ChunkPos> = self.pending_chunks.iter().copied().collect();

        // Sort by distance to player
        candidates.sort_by_key(|pos| {
            let dx = pos.0.x - player_chunk_pos.0.x;
            let dz = pos.0.y - player_chunk_pos.0.y;
            dx * dx + dz * dz
        });

        let mut chunks_to_send = Vec::new();

        for pos in candidates {
            if chunks_to_send.len() >= max_batch_size {
                break;
            }

            if let Some(holder) = world
                .chunk_map
                .chunks
                .read_sync(&pos, |_, chunk| chunk.clone())
                && holder.persisted_status() == Some(ChunkStatus::Full)
            {
                chunks_to_send.push(holder);
                self.pending_chunks.remove(&pos);
            }
        }
        chunks_to_send
    }

    /// Handles the acknowledgement of a chunk batch from the client.
    pub fn on_chunk_batch_received_by_client(&mut self, _batch_size: f32) {
        if self.unacknowledged_batches > 0 {
            self.unacknowledged_batches -= 1;
        }
    }

    /// Broadcasts light updates for chunks that have had their light modified by neighbors.
    fn broadcast_light_updates(
        connection: Arc<JavaConnection>,
        world: &World,
        player_chunk_pos: ChunkPos,
    ) {
        let view_distance = 10; // TODO: Get from player settings

        for dx in -view_distance..=view_distance {
            for dz in -view_distance..=view_distance {
                let pos = ChunkPos(steel_utils::math::Vector2::new(
                    player_chunk_pos.0.x + dx,
                    player_chunk_pos.0.y + dz,
                ));

                if let Some(holder) = world
                    .chunk_map
                    .chunks
                    .read_sync(&pos, |_, chunk| chunk.clone())
                {
                    // Check if this chunk has light changes and is already at Full status
                    if holder.has_light_changes()
                        && holder.persisted_status() == Some(ChunkStatus::Full)
                    {
                        // Get the changed section flags before clearing
                        let sky_changed = holder
                            .sky_changed_sections
                            .load(std::sync::atomic::Ordering::Relaxed);
                        let block_changed = holder
                            .block_changed_sections
                            .load(std::sync::atomic::Ordering::Relaxed);

                        // Only send if there are actual changes
                        if (sky_changed != 0 || block_changed != 0)
                            && let Some(chunk_lock) = holder.try_chunk(ChunkStatus::Full)
                        {
                            if let Some(chunk_arc) = chunk_lock.as_ref()
                                && let ChunkAccess::Full(level_chunk) = chunk_arc.as_ref()
                            {
                                // Extract only the changed light sections
                                let light_data = level_chunk
                                    .extract_changed_light_data(sky_changed, block_changed);

                                // Verify arrays match masks before sending
                                if !light_data.sky_updates.is_empty()
                                    || !light_data.block_updates.is_empty()
                                    || (light_data.empty_sky_y_mask.0.first().map_or(0, |v| *v)
                                        != 0)
                                    || (light_data.empty_block_y_mask.0.first().map_or(0, |v| *v)
                                        != 0)
                                {
                                    let light_update = CLightUpdate { pos, light_data };
                                    connection.send_packet(light_update);
                                }

                                // Clear the changed sections now that we've sent the update
                                holder.clear_light_changes();
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Default for ChunkSender {
    fn default() -> Self {
        Self {
            pending_chunks: FxHashSet::default(),
            unacknowledged_batches: 0,
            desired_chunks_per_tick: 32.0,
            batch_quota: 0.0,
            max_unacknowledged_batches: 1,
        }
    }
}
