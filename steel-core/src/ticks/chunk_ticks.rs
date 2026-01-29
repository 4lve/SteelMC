//! Per-chunk scheduled tick storage.

use std::collections::BinaryHeap;
use std::hash::Hash;
use std::mem;

use rustc_hash::FxHashSet;
use steel_utils::BlockPos;

use super::{ScheduledTick, TickKey};

/// Per-chunk storage for scheduled ticks.
///
/// This struct manages scheduled ticks for a single chunk, providing:
/// - A priority queue ordered by trigger time
/// - Deduplication to prevent scheduling the same (pos, type) twice
///
/// When a tick is scheduled for a position that already has a pending tick,
/// the new tick is ignored and the existing one keeps its timing.
pub struct LevelChunkTicks<T: Copy + Eq + Hash> {
    /// Priority queue of scheduled ticks, ordered by trigger time.
    tick_queue: BinaryHeap<ScheduledTick<T>>,
    /// Set of (pos, type) pairs for deduplication.
    /// If a key exists here, that position+type already has a scheduled tick.
    ticks_per_position: FxHashSet<TickKey<T>>,
}

impl<T: Copy + Eq + Hash> LevelChunkTicks<T> {
    /// Creates a new empty chunk tick container.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tick_queue: BinaryHeap::new(),
            ticks_per_position: FxHashSet::default(),
        }
    }

    /// Schedules a tick if one isn't already scheduled for this (pos, type).
    ///
    /// Returns `true` if the tick was scheduled, `false` if one already exists.
    pub fn schedule(&mut self, tick: ScheduledTick<T>) -> bool {
        let key = TickKey::from(&tick);
        if self.ticks_per_position.insert(key) {
            self.tick_queue.push(tick);
            true
        } else {
            // Already scheduled, keep existing timing
            false
        }
    }

    /// Returns a reference to the next tick to fire, without removing it.
    #[must_use]
    pub fn peek(&self) -> Option<&ScheduledTick<T>> {
        self.tick_queue.peek()
    }

    /// Removes and returns the next tick to fire.
    ///
    /// Also removes it from the deduplication set.
    pub fn poll(&mut self) -> Option<ScheduledTick<T>> {
        if let Some(tick) = self.tick_queue.pop() {
            let key = TickKey::from(&tick);
            self.ticks_per_position.remove(&key);
            Some(tick)
        } else {
            None
        }
    }

    /// Checks if a tick is scheduled for the given position and type.
    pub fn has_scheduled_tick(&self, pos: BlockPos, tick_type: T) -> bool {
        self.ticks_per_position
            .contains(&TickKey { pos, tick_type })
    }

    /// Returns the number of scheduled ticks in this chunk.
    #[must_use]
    pub fn count(&self) -> usize {
        self.tick_queue.len()
    }

    /// Returns `true` if there are no scheduled ticks.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tick_queue.is_empty()
    }

    /// Removes all ticks matching the predicate.
    pub fn remove_if<F>(&mut self, mut predicate: F)
    where
        F: FnMut(&ScheduledTick<T>) -> bool,
    {
        // Drain and rebuild the queue, keeping only non-matching ticks
        let old_queue = mem::take(&mut self.tick_queue);
        for tick in old_queue {
            if predicate(&tick) {
                let key = TickKey::from(&tick);
                self.ticks_per_position.remove(&key);
            } else {
                self.tick_queue.push(tick);
            }
        }
    }

    /// Returns an iterator over all scheduled ticks.
    ///
    /// Note: The order is not guaranteed to match trigger order.
    /// Used by `/clone` command to copy ticks between areas.
    pub fn iter(&self) -> impl Iterator<Item = &ScheduledTick<T>> {
        self.tick_queue.iter()
    }
}

impl<T: Copy + Eq + Hash> Default for LevelChunkTicks<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticks::TickPriority;
    use steel_utils::math::Vector3;

    #[test]
    fn test_schedule_and_poll() {
        let mut ticks: LevelChunkTicks<u32> = LevelChunkTicks::new();
        let pos = BlockPos(Vector3::new(10, 64, 20));

        // Schedule a tick
        let tick = ScheduledTick::new(1, pos, 100, 0);
        assert!(ticks.schedule(tick));
        assert_eq!(ticks.count(), 1);

        // Try to schedule same pos+type again - should be ignored
        let duplicate = ScheduledTick::new(1, pos, 200, 1);
        assert!(!ticks.schedule(duplicate));
        assert_eq!(ticks.count(), 1);

        // Poll should return the first tick (at tick 100, not 200)
        let polled = ticks.poll().expect("should have a tick");
        assert_eq!(polled.trigger_tick, 100);
        assert_eq!(ticks.count(), 0);

        // Now we can schedule again
        let new_tick = ScheduledTick::new(1, pos, 300, 2);
        assert!(ticks.schedule(new_tick));
        assert_eq!(ticks.count(), 1);
    }

    #[test]
    fn test_ordering() {
        let mut ticks: LevelChunkTicks<u32> = LevelChunkTicks::new();

        // Schedule ticks at different times
        ticks.schedule(ScheduledTick::new(
            1,
            BlockPos(Vector3::new(0, 0, 0)),
            200,
            0,
        ));
        ticks.schedule(ScheduledTick::new(
            2,
            BlockPos(Vector3::new(1, 0, 0)),
            100,
            1,
        ));
        ticks.schedule(ScheduledTick::new(
            3,
            BlockPos(Vector3::new(2, 0, 0)),
            150,
            2,
        ));

        // Should come out in order: 100, 150, 200
        assert_eq!(ticks.poll().expect("first").trigger_tick, 100);
        assert_eq!(ticks.poll().expect("second").trigger_tick, 150);
        assert_eq!(ticks.poll().expect("third").trigger_tick, 200);
    }

    #[test]
    fn test_priority_ordering() {
        let mut ticks: LevelChunkTicks<u32> = LevelChunkTicks::new();

        // Schedule ticks at same time but different priorities
        ticks.schedule(ScheduledTick::with_priority(
            1,
            BlockPos(Vector3::new(0, 0, 0)),
            100,
            TickPriority::Low,
            0,
        ));
        ticks.schedule(ScheduledTick::with_priority(
            2,
            BlockPos(Vector3::new(1, 0, 0)),
            100,
            TickPriority::High,
            1,
        ));
        ticks.schedule(ScheduledTick::with_priority(
            3,
            BlockPos(Vector3::new(2, 0, 0)),
            100,
            TickPriority::Normal,
            2,
        ));

        // Should come out: High, Normal, Low
        assert_eq!(ticks.poll().expect("high").priority, TickPriority::High);
        assert_eq!(ticks.poll().expect("normal").priority, TickPriority::Normal);
        assert_eq!(ticks.poll().expect("low").priority, TickPriority::Low);
    }
}
