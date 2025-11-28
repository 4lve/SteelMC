//! This module is responsible for sending chunks to the client.
use std::{collections::HashSet, sync::Arc};

use steel_protocol::packets::game::{
    CChunkBatchFinished, CChunkBatchStart, CForgetLevelChunk, CLevelChunkWithLight,
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
    pub pending_chunks: HashSet<ChunkPos>,
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
    pub fn send_next_chunks(
        &mut self,
        connection: Arc<JavaConnection>,
        world: &World,
        player_chunk_pos: ChunkPos,
    ) {
        if self.unacknowledged_batches < self.max_unacknowledged_batches {
            let max_batch_size = self.desired_chunks_per_tick.max(1.0);
            self.batch_quota =
                (self.batch_quota + self.desired_chunks_per_tick).min(max_batch_size);

            if self.batch_quota >= 1.0 && !self.pending_chunks.is_empty() {
                let chunks_to_process = self.collect_candidates(world, player_chunk_pos);
                if !chunks_to_process.is_empty() {
                    self.unacknowledged_batches += 1;
                    self.batch_quota -= chunks_to_process.len() as f32;

                    let _ = spawn_blocking(move || {
                        let mut chunks_to_send = Vec::new();
                        for holder in chunks_to_process {
                            if let Some(chunk) = holder.try_chunk(ChunkStatus::Full).map(|chunk| {
                                let chunk = chunk.read();
                                match &*chunk {
                                    Some(ChunkAccess::Full(chunk)) => CLevelChunkWithLight {
                                        pos: holder.get_pos(),
                                        chunk_data: chunk.extract_chunk_data(),
                                        light_data: chunk.extract_light_data(),
                                    },
                                    _ => unreachable!(),
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

            if let Some(holder) = world.chunk_map.chunks.get_sync(&pos) {
                let holder = holder.get().clone();
                if holder.persisted_status() == Some(ChunkStatus::Full) {
                    chunks_to_send.push(holder);
                    self.pending_chunks.remove(&pos);
                }
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
}

impl Default for ChunkSender {
    fn default() -> Self {
        Self {
            pending_chunks: HashSet::new(),
            unacknowledged_batches: 0,
            desired_chunks_per_tick: 32.0,
            batch_quota: 0.0,
            max_unacknowledged_batches: 1,
        }
    }
}
