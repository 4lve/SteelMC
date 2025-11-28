use indexmap::IndexSet;
use rustc_hash::{FxBuildHasher, FxHashMap};
use std::cmp::min;
use steel_utils::ChunkPos;

/// A standard max level for chunks that are unloaded.
pub const MAX_LEVEL: u8 = 45;
/// The level count (`MAX_LEVEL` + 1).
const LEVEL_COUNT: usize = (MAX_LEVEL + 1) as usize;

/// Returns all 8 neighbors of a chunk position (including diagonals).
/// This matches Java's `ChunkTracker` which uses a 3x3 grid around the chunk.
#[inline]
fn get_neighbor_keys(pos: i64) -> [i64; 8] {
    let chunk = ChunkPos::from_i64(pos);
    let x = chunk.0.x;
    let z = chunk.0.y;
    [
        ChunkPos::new(x - 1, z - 1).as_i64(),
        ChunkPos::new(x, z - 1).as_i64(),
        ChunkPos::new(x + 1, z - 1).as_i64(),
        ChunkPos::new(x - 1, z).as_i64(),
        ChunkPos::new(x + 1, z).as_i64(),
        ChunkPos::new(x - 1, z + 1).as_i64(),
        ChunkPos::new(x, z + 1).as_i64(),
        ChunkPos::new(x + 1, z + 1).as_i64(),
    ]
}

/// Tracks chunk levels based on propagation.
///
/// This implementation matches Java's `DynamicGraphMinFixedPoint` architecture:
/// - Uses `IndexSet<i64>` per priority level (like Java's `LongLinkedOpenHashSet`)
/// - O(1) dequeue by key, automatic deduplication
/// - No generation counters needed
pub struct ChunkTracker {
    /// Map of chunk positions to their current levels.
    levels: FxHashMap<i64, u8>,
    /// Priority queue: array of sets, one per priority level (0 to `MAX_LEVEL`).
    /// Using `IndexSet` for O(1) removal and insertion-order iteration.
    queues: Vec<IndexSet<i64, FxBuildHasher>>,
    /// Computed levels for nodes currently in the queue.
    /// Matches Java's `Long2ByteMap computedLevels`.
    computed_levels: FxHashMap<i64, u8>,
    /// Index of the first non-empty queue (optimization from Java).
    first_queued_level: usize,
}

impl Default for ChunkTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkTracker {
    /// Creates a new chunk tracker.
    #[must_use]
    pub fn new() -> Self {
        // Initialize with one IndexSet per priority level
        let mut queues = Vec::with_capacity(LEVEL_COUNT);
        for _ in 0..LEVEL_COUNT {
            queues.push(IndexSet::default());
        }

        Self {
            levels: FxHashMap::default(),
            queues,
            computed_levels: FxHashMap::default(),
            first_queued_level: LEVEL_COUNT,
        }
    }

    /// Gets the level of a chunk. Returns `MAX_LEVEL` if untracked.
    #[must_use]
    #[inline]
    pub fn get_level(&self, pos: ChunkPos) -> u8 {
        self.get_level_raw(pos.as_i64())
    }

    /// Gets the level by raw i64 key.
    #[inline]
    fn get_level_raw(&self, key: i64) -> u8 {
        self.levels.get(&key).copied().unwrap_or(MAX_LEVEL)
    }

    /// Updates the level of a chunk from a source (ticket).
    /// Matches Java's `ChunkTracker.update(node, newLevelFrom, onlyDecreased)`.
    pub fn update(
        &mut self,
        pos: ChunkPos,
        new_ticket_level: u8,
        get_ticket_level: impl Fn(ChunkPos) -> u8,
    ) {
        let key = pos.as_i64();
        let level_to = self.get_level_raw(key);
        let stored_computed = self.computed_levels.get(&key).copied();

        // Match Java's checkEdge logic
        let was_consistent = stored_computed.is_none();
        let old_computed = stored_computed.unwrap_or(level_to);

        // For ticket updates, compute the new level (onlyDecreased = false style)
        let new_computed = min(
            self.compute_level_raw(key, i64::MAX, new_ticket_level, &get_ticket_level),
            MAX_LEVEL,
        );

        if level_to != new_computed {
            let old_priority = Self::calculate_priority(level_to, old_computed);
            let new_priority = Self::calculate_priority(level_to, new_computed);

            if old_priority != new_priority && !was_consistent {
                self.dequeue_node(key, old_priority, new_priority);
            }

            self.enqueue_node(key, new_priority);
            self.computed_levels.insert(key, new_computed);
        } else if !was_consistent {
            let old_priority = Self::calculate_priority(level_to, old_computed);
            self.dequeue_node(key, old_priority, LEVEL_COUNT);
            self.computed_levels.remove(&key);
        }
    }

    /// Calculates priority for queue placement.
    #[inline]
    fn calculate_priority(level: u8, computed_level: u8) -> usize {
        min(min(level, computed_level), MAX_LEVEL) as usize
    }

    /// Removes a node from its current queue position.
    fn dequeue_node(&mut self, key: i64, from_priority: usize, upper_bound: usize) {
        let queue = &mut self.queues[from_priority];
        queue.shift_remove(&key);

        if queue.is_empty() && self.first_queued_level == from_priority {
            self.update_first_queued_level(upper_bound);
        }
    }

    /// Adds a node to a queue at the given priority.
    fn enqueue_node(&mut self, key: i64, priority: usize) {
        self.queues[priority].insert(key);
        if self.first_queued_level > priority {
            self.first_queued_level = priority;
        }
    }

    /// Updates `first_queued_level` to the next non-empty queue.
    fn update_first_queued_level(&mut self, upper_bound: usize) {
        let old_level = self.first_queued_level;
        self.first_queued_level = upper_bound;

        for i in (old_level + 1)..upper_bound {
            if !self.queues[i].is_empty() {
                self.first_queued_level = i;
                break;
            }
        }
    }

    /// Checks if the queue is empty.
    #[inline]
    fn is_empty(&self) -> bool {
        self.first_queued_level >= LEVEL_COUNT
    }

    /// Removes and returns the first node from the lowest priority queue.
    fn pop_first(&mut self) -> Option<i64> {
        if self.is_empty() {
            return None;
        }

        let queue = &mut self.queues[self.first_queued_level];
        let key = queue.shift_remove_index(0)?;

        if queue.is_empty() {
            self.update_first_queued_level(LEVEL_COUNT);
        }

        Some(key)
    }

    /// Processes all pending updates in the queue.
    /// Returns changes as (pos, `old_level`, `new_level`).
    #[inline]
    pub fn process_all_updates(
        &mut self,
        get_ticket_level: impl Fn(ChunkPos) -> u8,
    ) -> Vec<(ChunkPos, u8, u8)> {
        let mut changes = Vec::new();

        while let Some(key) = self.pop_first() {
            let level = min(self.get_level_raw(key), MAX_LEVEL);
            let computed_level = self.computed_levels.remove(&key).unwrap_or(MAX_LEVEL);

            if computed_level < level {
                // Level is decreasing
                self.set_level_raw(key, computed_level);
                changes.push((ChunkPos::from_i64(key), level, computed_level));
                self.check_neighbors_after_update(key, computed_level, true, &get_ticket_level);
            } else if computed_level > level {
                // Level is increasing - set to MAX first
                self.set_level_raw(key, MAX_LEVEL);
                changes.push((ChunkPos::from_i64(key), level, MAX_LEVEL));

                // Re-enqueue if not yet at final level
                if computed_level != MAX_LEVEL {
                    let priority = Self::calculate_priority(MAX_LEVEL, computed_level);
                    self.enqueue_node(key, priority);
                    self.computed_levels.insert(key, computed_level);
                }

                self.check_neighbors_after_update(key, level, false, &get_ticket_level);
            }
        }

        changes
    }

    /// Sets the level for a chunk by raw key.
    #[inline]
    fn set_level_raw(&mut self, key: i64, level: u8) {
        if level >= MAX_LEVEL {
            self.levels.remove(&key);
        } else {
            self.levels.insert(key, level);
        }
    }

    /// Checks and updates neighbors after a level change.
    fn check_neighbors_after_update(
        &mut self,
        key: i64,
        level: u8,
        only_decrease: bool,
        get_ticket_level: &impl Fn(ChunkPos) -> u8,
    ) {
        // Optimization from Java: skip if only decreasing and level is near max
        if only_decrease && level >= MAX_LEVEL - 1 {
            return;
        }

        for neighbor_key in get_neighbor_keys(key) {
            self.check_neighbor(key, neighbor_key, level, only_decrease, get_ticket_level);
        }
    }

    /// Checks a specific neighbor and enqueues it if needed.
    /// Matches Java's `checkNeighbor` method.
    fn check_neighbor(
        &mut self,
        from: i64,
        to: i64,
        from_level: u8,
        only_decrease: bool,
        get_ticket_level: &impl Fn(ChunkPos) -> u8,
    ) {
        let stored_computed = self.computed_levels.get(&to).copied();
        let level_from = min(from_level + 1, MAX_LEVEL); // computeLevelFromNeighbor equivalent

        if only_decrease {
            let level_to = self.get_level_raw(to);
            self.check_edge(
                from,
                to,
                level_from,
                level_to,
                stored_computed,
                true,
                get_ticket_level,
            );
        } else {
            let was_consistent = stored_computed.is_none();
            let old_computed = if was_consistent {
                min(self.get_level_raw(to), MAX_LEVEL)
            } else {
                stored_computed.unwrap()
            };

            if level_from == old_computed {
                let level_to = if was_consistent {
                    old_computed
                } else {
                    self.get_level_raw(to)
                };
                self.check_edge(
                    from,
                    to,
                    MAX_LEVEL,
                    level_to,
                    stored_computed,
                    false,
                    get_ticket_level,
                );
            }
        }
    }

    /// Core edge checking logic from Java's `DynamicGraphMinFixedPoint.checkEdge`.
    fn check_edge(
        &mut self,
        from: i64,
        to: i64,
        new_level_from: u8,
        level_to: u8,
        stored_computed: Option<u8>,
        only_decreased: bool,
        get_ticket_level: &impl Fn(ChunkPos) -> u8,
    ) {
        let new_level_from = min(new_level_from, MAX_LEVEL);
        let level_to = min(level_to, MAX_LEVEL);
        let was_consistent = stored_computed.is_none();
        let old_computed = stored_computed.unwrap_or(level_to);

        let new_computed = if only_decreased {
            min(old_computed, new_level_from)
        } else {
            min(
                self.compute_level_raw(to, from, new_level_from, get_ticket_level),
                MAX_LEVEL,
            )
        };

        let old_priority = Self::calculate_priority(level_to, old_computed);

        if level_to != new_computed {
            let new_priority = Self::calculate_priority(level_to, new_computed);
            if old_priority != new_priority && !was_consistent {
                self.dequeue_node(to, old_priority, new_priority);
            }
            self.enqueue_node(to, new_priority);
            self.computed_levels.insert(to, new_computed);
        } else if !was_consistent {
            self.dequeue_node(to, old_priority, LEVEL_COUNT);
            self.computed_levels.remove(&to);
        }
    }

    /// Computes the level for a node based on all neighbors and ticket level.
    /// Matches Java's `getComputedLevel`.
    fn compute_level_raw(
        &self,
        key: i64,
        known_parent: i64,
        known_level_from_parent: u8,
        get_ticket_level: &impl Fn(ChunkPos) -> u8,
    ) -> u8 {
        let pos = ChunkPos::from_i64(key);
        let ticket_level = get_ticket_level(pos);
        let mut computed_level = min(known_level_from_parent, ticket_level);

        if computed_level == 0 {
            return 0;
        }

        for neighbor_key in get_neighbor_keys(key) {
            // Check if neighbor is the node itself (becomes source check)
            let effective_neighbor = if neighbor_key == key {
                i64::MAX // SOURCE
            } else {
                neighbor_key
            };

            if effective_neighbor != known_parent {
                let cost = if effective_neighbor == i64::MAX {
                    get_ticket_level(pos) // getLevelFromSource
                } else {
                    min(self.get_level_raw(effective_neighbor) + 1, MAX_LEVEL)
                };

                if computed_level > cost {
                    computed_level = cost;
                }

                if computed_level == 0 {
                    return 0;
                }
            }
        }

        computed_level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticket_propagation() {
        let mut tracker = ChunkTracker::new();
        let pos = ChunkPos::new(0, 0);

        // Add ticket
        tracker.update(pos, 31, |_| MAX_LEVEL);
        tracker.process_all_updates(|p| if p == pos { 31 } else { MAX_LEVEL });

        assert_eq!(tracker.get_level(pos), 31);
        assert_eq!(tracker.get_level(ChunkPos::new(1, 0)), 32);
    }

    #[test]
    fn test_ticket_removal() {
        let mut tracker = ChunkTracker::new();
        let pos = ChunkPos::new(0, 0);

        // Setup initial state
        tracker.update(pos, 31, |_| MAX_LEVEL);
        tracker.process_all_updates(|p| if p == pos { 31 } else { MAX_LEVEL });

        // Remove ticket
        tracker.update(pos, MAX_LEVEL, |_| MAX_LEVEL);
        tracker.process_all_updates(|_| MAX_LEVEL);

        assert_eq!(tracker.get_level(pos), MAX_LEVEL);
        assert_eq!(tracker.get_level(ChunkPos::new(1, 0)), MAX_LEVEL);
    }

    #[test]
    fn test_circular_dependency_unloading() {
        let mut tracker = ChunkTracker::new();
        let center = ChunkPos::new(0, 0);
        let neighbor = ChunkPos::new(1, 0);

        // Setup: Center has ticket 31. Neighbor has ticket 33 (weaker).
        // Center -> 31. Neighbor -> 32 (from center).
        tracker.update(center, 31, |_| MAX_LEVEL);
        tracker.update(neighbor, 33, |_| MAX_LEVEL);

        tracker.process_all_updates(|p| {
            if p == center {
                31
            } else if p == neighbor {
                33
            } else {
                MAX_LEVEL
            }
        });

        assert_eq!(tracker.get_level(center), 31);
        assert_eq!(tracker.get_level(neighbor), 32); // Propagated from center is better than 33

        // Remove center ticket. Neighbor ticket remains 33.
        // Center should become 34 (from neighbor 33). Neighbor becomes 33 (its ticket).
        tracker.update(center, MAX_LEVEL, |_| MAX_LEVEL);
        tracker.process_all_updates(|p| if p == neighbor { 33 } else { MAX_LEVEL });

        assert_eq!(tracker.get_level(neighbor), 33);
        assert_eq!(tracker.get_level(center), 34);
    }

    #[test]
    fn test_circular_dependency_full_unload() {
        let mut tracker = ChunkTracker::new();
        let center = ChunkPos::new(0, 0);
        let neighbor = ChunkPos::new(1, 0);

        // Setup: Center has ticket 31.
        tracker.update(center, 31, |_| MAX_LEVEL);
        tracker.process_all_updates(|p| if p == center { 31 } else { MAX_LEVEL });

        assert_eq!(tracker.get_level(center), 31);
        assert_eq!(tracker.get_level(neighbor), 32);

        // Remove ticket. Both should unload.
        tracker.update(center, MAX_LEVEL, |_| MAX_LEVEL);
        tracker.process_all_updates(|_| MAX_LEVEL);

        assert_eq!(tracker.get_level(center), MAX_LEVEL);
        assert_eq!(tracker.get_level(neighbor), MAX_LEVEL);
    }

    #[test]
    fn test_diagonal_propagation() {
        let mut tracker = ChunkTracker::new();
        let center = ChunkPos::new(0, 0);

        // Add ticket at center
        tracker.update(center, 31, |_| MAX_LEVEL);
        tracker.process_all_updates(|p| if p == center { 31 } else { MAX_LEVEL });

        assert_eq!(tracker.get_level(center), 31);

        // Cardinal neighbors should be level 32
        assert_eq!(tracker.get_level(ChunkPos::new(1, 0)), 32);
        assert_eq!(tracker.get_level(ChunkPos::new(0, 1)), 32);

        // Diagonal neighbors should ALSO be level 32 (not 33 like with 4-neighbor propagation)
        // This is the key test - Java uses 8-neighbor propagation
        assert_eq!(tracker.get_level(ChunkPos::new(1, 1)), 32);
        assert_eq!(tracker.get_level(ChunkPos::new(-1, -1)), 32);
        assert_eq!(tracker.get_level(ChunkPos::new(1, -1)), 32);
        assert_eq!(tracker.get_level(ChunkPos::new(-1, 1)), 32);

        // Chunks 2 away diagonally should be level 33
        assert_eq!(tracker.get_level(ChunkPos::new(2, 2)), 33);
    }
}
