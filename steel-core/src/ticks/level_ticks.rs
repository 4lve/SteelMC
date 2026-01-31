//! World-level scheduled tick coordinator.

use std::collections::BinaryHeap;
use std::hash::Hash;

use rustc_hash::FxHashMap;
use steel_utils::{BlockPos, ChunkPos};

use super::{LevelChunkTicks, ScheduledTick, TickPriority};

/// Converts a block position to its containing chunk position.
#[inline]
fn chunk_pos_from_block(pos: &BlockPos) -> ChunkPos {
    ChunkPos::new(pos.0.x >> 4, pos.0.z >> 4)
}

/// World-level coordinator for scheduled ticks.
///
/// This struct manages all scheduled ticks across all loaded chunks,
/// providing efficient lookup of which chunks have pending ticks and
/// coordinating tick processing each game tick.
///
/// # Architecture
///
/// - Each loaded chunk has a `LevelChunkTicks` stored in `all_containers`
/// - `next_tick_for_container` tracks the earliest trigger tick per chunk
///   for efficient filtering during tick processing
/// - During `tick()`, we collect all due ticks and run them
pub struct LevelTicks<T: Copy + Eq + Hash> {
    /// Map of chunk position to chunk tick container.
    all_containers: FxHashMap<ChunkPos, LevelChunkTicks<T>>,
    /// Tracks the earliest scheduled tick for each chunk.
    /// Used to efficiently skip chunks with no due ticks.
    next_tick_for_container: FxHashMap<ChunkPos, u64>,
    /// Counter for generating unique sub-tick order values.
    sub_tick_counter: u64,
}

impl<T: Copy + Eq + Hash> LevelTicks<T> {
    /// Creates a new empty world tick coordinator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            all_containers: FxHashMap::default(),
            next_tick_for_container: FxHashMap::default(),
            sub_tick_counter: 0,
        }
    }

    /// Adds a chunk's tick container when the chunk is loaded.
    ///
    /// If the container has pending ticks, updates the next-tick tracking.
    pub fn add_container(&mut self, pos: ChunkPos, container: LevelChunkTicks<T>) {
        if let Some(next_tick) = container.peek() {
            self.next_tick_for_container
                .insert(pos, next_tick.trigger_tick);
        }
        self.all_containers.insert(pos, container);
    }

    /// Removes a chunk's tick container when the chunk is unloaded.
    ///
    /// Returns the container so it can be saved if needed.
    pub fn remove_container(&mut self, pos: &ChunkPos) -> Option<LevelChunkTicks<T>> {
        self.next_tick_for_container.remove(pos);
        self.all_containers.remove(pos)
    }

    /// Schedules a tick at the given position.
    ///
    /// The tick will fire at `current_tick + delay`.
    pub fn schedule(
        &mut self,
        pos: BlockPos,
        tick_type: T,
        current_tick: u64,
        delay: u32,
        priority: TickPriority,
    ) -> bool {
        let chunk_pos = chunk_pos_from_block(&pos);
        let trigger_tick = current_tick + u64::from(delay);

        let Some(container) = self.all_containers.get_mut(&chunk_pos) else {
            log::warn!("Attempted to schedule tick in unloaded chunk {chunk_pos:?}");
            return false;
        };

        let sub_tick_order = self.sub_tick_counter;
        self.sub_tick_counter += 1;

        let tick =
            ScheduledTick::with_priority(tick_type, pos, trigger_tick, priority, sub_tick_order);

        if container.schedule(tick) {
            // Update next-tick tracking if this is earlier than current earliest
            self.next_tick_for_container
                .entry(chunk_pos)
                .and_modify(|earliest| {
                    if trigger_tick < *earliest {
                        *earliest = trigger_tick;
                    }
                })
                .or_insert(trigger_tick);
            true
        } else {
            false
        }
    }

    /// Schedules a tick with normal priority.
    pub fn schedule_tick(
        &mut self,
        pos: BlockPos,
        tick_type: T,
        current_tick: u64,
        delay: u32,
    ) -> bool {
        self.schedule(pos, tick_type, current_tick, delay, TickPriority::Normal)
    }

    /// Checks if a tick is already scheduled for the given position and type.
    #[must_use]
    pub fn has_scheduled_tick(&self, pos: BlockPos, tick_type: T) -> bool {
        let chunk_pos = chunk_pos_from_block(&pos);
        self.all_containers
            .get(&chunk_pos)
            .is_some_and(|c| c.has_scheduled_tick(pos, tick_type))
    }

    /// Processes all ticks that are due at or before `current_tick`.
    ///
    /// Returns a vec of (position, type) pairs for ticks that fired.
    /// The callback should be used to actually execute the tick behavior.
    ///
    /// # Arguments
    /// * `current_tick` - The current game tick
    /// * `max_ticks` - Maximum number of ticks to process this call
    /// * `can_tick_chunk` - Predicate to check if a chunk should be ticked
    ///
    /// # Panics
    /// Panics if the internal tick queue becomes inconsistent (peek returns Some
    /// but poll returns None). This should never happen in practice.
    #[must_use]
    pub fn tick<F>(
        &mut self,
        current_tick: u64,
        max_ticks: usize,
        can_tick_chunk: F,
    ) -> Vec<(BlockPos, T)>
    where
        F: Fn(&ChunkPos) -> bool,
    {
        // Collect chunks that have due ticks
        let mut chunks_to_tick: Vec<ChunkPos> = self
            .next_tick_for_container
            .iter()
            .filter(|(pos, earliest)| **earliest <= current_tick && can_tick_chunk(pos))
            .map(|(pos, _)| *pos)
            .collect();

        // Sort by earliest tick for consistent ordering
        chunks_to_tick.sort_by_key(|pos| self.next_tick_for_container.get(pos).copied());

        let mut result = Vec::new();
        let mut ticks_processed = 0;

        // Use a temporary heap to merge ticks from multiple chunks in order
        let mut merged_heap: BinaryHeap<(ScheduledTick<T>, ChunkPos)> = BinaryHeap::new();

        // Seed the heap with the first tick from each chunk
        for chunk_pos in &chunks_to_tick {
            if let Some(container) = self.all_containers.get(chunk_pos)
                && let Some(tick) = container.peek()
                && tick.trigger_tick <= current_tick
            {
                merged_heap.push((tick.clone(), *chunk_pos));
            }
        }

        // Process ticks in global order
        while ticks_processed < max_ticks {
            let Some((tick, chunk_pos)) = merged_heap.pop() else {
                break;
            };

            // Double-check the tick is still due (could have been removed)
            let Some(container) = self.all_containers.get_mut(&chunk_pos) else {
                continue;
            };

            // Verify this is still the top tick and it matches
            if let Some(top) = container.peek()
                && top.pos == tick.pos
                && top.trigger_tick == tick.trigger_tick
            {
                // Actually remove and process (safe to unwrap - we just peeked)
                let tick = container
                    .poll()
                    .expect("container.peek() returned Some, poll() should too");
                result.push((tick.pos, tick.tick_type));
                ticks_processed += 1;

                // Add the next tick from this chunk to the heap
                if let Some(next) = container.peek()
                    && next.trigger_tick <= current_tick
                {
                    merged_heap.push((next.clone(), chunk_pos));
                }
            }
        }

        // Update next-tick tracking for affected chunks
        for chunk_pos in chunks_to_tick {
            if let Some(container) = self.all_containers.get(&chunk_pos) {
                if let Some(next) = container.peek() {
                    self.next_tick_for_container
                        .insert(chunk_pos, next.trigger_tick);
                } else {
                    self.next_tick_for_container.remove(&chunk_pos);
                }
            }
        }

        result
    }

    /// Returns the total number of scheduled ticks across all chunks.
    #[must_use]
    pub fn count(&self) -> usize {
        self.all_containers
            .values()
            .map(LevelChunkTicks::count)
            .sum()
    }
}

impl<T: Copy + Eq + Hash> Default for LevelTicks<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use steel_utils::math::{Vector2, Vector3};

    #[test]
    fn test_schedule_and_tick() {
        let mut level_ticks: LevelTicks<u32> = LevelTicks::new();

        // Add a chunk container
        let chunk_pos = ChunkPos(Vector2::new(0, 0));
        level_ticks.add_container(chunk_pos, LevelChunkTicks::new());

        // Schedule some ticks
        let pos1 = BlockPos(Vector3::new(5, 64, 5));
        let pos2 = BlockPos(Vector3::new(10, 64, 10));

        level_ticks.schedule_tick(pos1, 1, 100, 10); // fires at tick 110
        level_ticks.schedule_tick(pos2, 2, 100, 5); // fires at tick 105

        assert_eq!(level_ticks.count(), 2);

        // Tick at 104 - nothing should fire
        let fired = level_ticks.tick(104, 100, |_| true);
        assert!(fired.is_empty());

        // Tick at 105 - pos2 should fire
        let fired = level_ticks.tick(105, 100, |_| true);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].0, pos2);

        // Tick at 110 - pos1 should fire
        let fired = level_ticks.tick(110, 100, |_| true);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].0, pos1);

        assert_eq!(level_ticks.count(), 0);
    }

    #[test]
    fn test_deduplication() {
        let mut level_ticks: LevelTicks<u32> = LevelTicks::new();
        let chunk_pos = ChunkPos(Vector2::new(0, 0));
        level_ticks.add_container(chunk_pos, LevelChunkTicks::new());

        let pos = BlockPos(Vector3::new(5, 64, 5));

        // First schedule succeeds
        assert!(level_ticks.schedule_tick(pos, 1, 100, 10));
        // Second schedule for same pos+type fails
        assert!(!level_ticks.schedule_tick(pos, 1, 100, 20));

        assert_eq!(level_ticks.count(), 1);

        // The tick should fire at 110 (first scheduled), not 120
        let fired = level_ticks.tick(110, 100, |_| true);
        assert_eq!(fired.len(), 1);

        let fired = level_ticks.tick(120, 100, |_| true);
        assert!(fired.is_empty());
    }
}
