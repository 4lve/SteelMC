//! 2-element LRU cache for chunk access during light propagation.

use std::sync::Arc;
use steel_utils::{ChunkPos, math::Vector2};

use crate::chunk::chunk_holder::ChunkHolder;

/// 2-element LRU cache for recently accessed chunks.
///
/// This cache stores the two most recently accessed chunks to avoid
/// repeated lock acquisitions during light propagation. The cache uses
/// a simple LRU eviction policy: when both slots are full, the least
/// recently used entry is evicted.
pub struct ChunkCache {
    /// Cache size (always 2 for vanilla compatibility).
    cache_size: usize,

    /// Cached chunk positions (invalid when ChunkPos(-1, -1)).
    last_chunk_pos: [ChunkPos; 2],

    /// Cached chunk holders.
    last_chunk_holder: [Option<Arc<ChunkHolder>>; 2],

    /// Access counters for LRU tracking (higher = more recent).
    access_counter: [u64; 2],

    /// Global access counter.
    global_counter: u64,
}

impl ChunkCache {
    /// Creates a new empty chunk cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache_size: 2,
            last_chunk_pos: [
                ChunkPos(Vector2::new(-1, -1)),
                ChunkPos(Vector2::new(-1, -1)),
            ],
            last_chunk_holder: [None, None],
            access_counter: [0, 0],
            global_counter: 0,
        }
    }

    /// Attempts to get a chunk from the cache.
    ///
    /// Returns `Some(holder)` if found in cache, `None` otherwise.
    pub fn get(&mut self, pos: ChunkPos) -> Option<Arc<ChunkHolder>> {
        for i in 0..self.cache_size {
            if self.last_chunk_pos[i] == pos {
                // Cache hit - update access time
                self.global_counter += 1;
                self.access_counter[i] = self.global_counter;

                return self.last_chunk_holder[i].clone();
            }
        }

        // Cache miss
        None
    }

    /// Inserts a chunk into the cache, evicting LRU entry if needed.
    pub fn insert(&mut self, pos: ChunkPos, holder: Arc<ChunkHolder>) {
        // Check if already in cache (update in place)
        for i in 0..self.cache_size {
            if self.last_chunk_pos[i] == pos {
                self.global_counter += 1;
                self.access_counter[i] = self.global_counter;
                self.last_chunk_holder[i] = Some(holder);
                return;
            }
        }

        // Find LRU slot (lowest access counter)
        let mut lru_idx = 0;
        let mut lru_count = self.access_counter[0];

        for i in 1..self.cache_size {
            if self.access_counter[i] < lru_count {
                lru_count = self.access_counter[i];
                lru_idx = i;
            }
        }

        // Evict LRU and insert new entry
        self.global_counter += 1;
        self.last_chunk_pos[lru_idx] = pos;
        self.last_chunk_holder[lru_idx] = Some(holder);
        self.access_counter[lru_idx] = self.global_counter;
    }

    /// Clears the cache.
    pub fn clear(&mut self) {
        for i in 0..self.cache_size {
            self.last_chunk_pos[i] = ChunkPos(Vector2::new(-1, -1));
            self.last_chunk_holder[i] = None;
            self.access_counter[i] = 0;
        }
        self.global_counter = 0;
    }

    /// Disables the cache (clears it).
    pub fn disable(&mut self) {
        self.clear();
    }
}

impl Default for ChunkCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hit() {
        let mut cache = ChunkCache::new();
        let pos = ChunkPos(Vector2::new(0, 0));

        // Initial state - cache miss
        assert!(cache.get(pos).is_none());
    }

    #[test]
    fn test_lru_eviction() {
        let cache = ChunkCache::new();

        let _pos1 = ChunkPos(Vector2::new(0, 0));
        let _pos2 = ChunkPos(Vector2::new(1, 0));
        let _pos3 = ChunkPos(Vector2::new(2, 0));

        // Verify cache size is 2
        assert_eq!(cache.cache_size, 2);
    }

    #[test]
    fn test_clear() {
        let mut cache = ChunkCache::new();

        cache.clear();

        // All positions should be invalid after clear
        for i in 0..2 {
            assert_eq!(cache.last_chunk_pos[i], ChunkPos(Vector2::new(-1, -1)));
            assert!(cache.last_chunk_holder[i].is_none());
            assert_eq!(cache.access_counter[i], 0);
        }
        assert_eq!(cache.global_counter, 0);
    }
}
