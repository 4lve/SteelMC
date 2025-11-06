use std::sync::Arc;

use steel_utils::ChunkPos;

use crate::player::Player;

// any idea to make this shorter?
const MAX_UNACKNOWLEDGED_BATCHES: u16 = 1;

#[derive(Debug, Clone, Default)]
pub struct ChunkSender {
    pub pending_chunks: Vec<ChunkPos>,
    pub unacknowledged_batches: u16,
    pub desired_chunks_per_tick: f32,
    pub batch_quota: f32,
}

impl ChunkSender {
    pub fn send_next_chunks(&mut self, _player: &Arc<Player>) {
        if self.unacknowledged_batches < MAX_UNACKNOWLEDGED_BATCHES {
            let max_batch_size = self.desired_chunks_per_tick.max(1.0);
            self.batch_quota = max_batch_size.min(self.desired_chunks_per_tick + self.batch_quota);
        }
    }
}
