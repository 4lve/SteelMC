use std::sync::Arc;

use steel_protocol::packets::game::{CChunkBatchFinished, CChunkBatchStart, CLevelChunkWithLight};
use steel_utils::ChunkPos;

use crate::{
    chunk::{chunk_packet_data::ChunkPacketData, level_chunk::LevelChunk},
    player::{Player, networking::JavaConnection},
};

// any idea to make this shorter?
const MAX_UNACKNOWLEDGED_BATCHES: u16 = 1;

#[derive(Debug, Clone)]
pub struct ChunkSender {
    pub pending_chunks: Vec<ChunkPos>,
    pub unacknowledged_batches: u16,
    pub desired_chunks_per_tick: f32,
    pub batch_quota: f32,
}

impl ChunkSender {
    pub fn send_next_chunks(&mut self, player: &Arc<Player>) {
        if self.unacknowledged_batches < MAX_UNACKNOWLEDGED_BATCHES {
            let max_batch_size = self.desired_chunks_per_tick.max(1.0);
            self.batch_quota = max_batch_size.min(self.desired_chunks_per_tick + self.batch_quota);

            if self.batch_quota >= 1.0 && !self.pending_chunks.is_empty() {
                //TODO! make it get the chunks
                let chunks_to_send: Vec<LevelChunk> = Vec::new();
                if !chunks_to_send.is_empty() {
                    self.unacknowledged_batches += 1;

                    let connection = &player.connection;
                    connection.send_packet(CChunkBatchStart {});

                    for chunk in &chunks_to_send {
                        Self::send_chunk(chunk, connection);
                    }

                    connection.send_packet(CChunkBatchFinished {
                        batch_size: chunks_to_send.len() as _,
                    });
                    self.batch_quota -= chunks_to_send.len() as f32;
                }
            }
        }
    }

    pub fn send_chunk(chunk: &LevelChunk, connection: &JavaConnection) {
        let chunk_data = ChunkPacketData { chunk }.extract_chunk_data();
        connection.send_packet(CLevelChunkWithLight {
            pos: chunk.pos,
            heightmaps: (),
            chunk_data,
        });
    }
}

impl Default for ChunkSender {
    fn default() -> Self {
        Self {
            pending_chunks: Vec::default(),
            unacknowledged_batches: 0,
            desired_chunks_per_tick: 9.0,
            batch_quota: 0.0,
        }
    }
}
