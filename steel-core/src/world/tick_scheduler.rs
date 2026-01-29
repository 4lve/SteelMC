//! Tick scheduler implementation.
//!
//! Based on Minecraft's `LevelTicks` and `ScheduledTick` system.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

use steel_utils::BlockPos;

/// Type of scheduled tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TickType {
    /// A block tick (e.g., redstone, crops growing).
    Block,
    /// A fluid tick (e.g., water/lava spreading).
    Fluid,
}

/// A scheduled tick entry.
///
/// Represents a pending tick for a block or fluid at a specific position.
#[derive(Debug, Clone)]
pub struct ScheduledTick {
    /// The position of the tick.
    pub pos: BlockPos,
    /// The type of tick (block or fluid).
    pub tick_type: TickType,
    /// The game tick when this should trigger.
    pub trigger_tick: u64,
    /// Priority for ordering (lower = higher priority).
    /// Used when multiple ticks are scheduled for the same tick.
    pub priority: i32,
    /// Unique sequence number to maintain insertion order for equal priorities.
    sequence: u64,
}

impl ScheduledTick {
    /// Creates a new scheduled tick.
    pub fn new(pos: BlockPos, tick_type: TickType, trigger_tick: u64, priority: i32, sequence: u64) -> Self {
        Self {
            pos,
            tick_type,
            trigger_tick,
            priority,
            sequence,
        }
    }
}

impl PartialEq for ScheduledTick {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos && self.tick_type == other.tick_type
    }
}

impl Eq for ScheduledTick {}

impl std::hash::Hash for ScheduledTick {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pos.hash(state);
        self.tick_type.hash(state);
    }
}

// Implement Ord for BinaryHeap (min-heap behavior via Reverse-like logic)
impl Ord for ScheduledTick {
    fn cmp(&self, other: &Self) -> Ordering {
        // Earlier trigger_tick = higher priority (should come first)
        // Lower priority value = higher priority
        // Earlier sequence = higher priority (FIFO for equal)
        other.trigger_tick.cmp(&self.trigger_tick)
            .then_with(|| other.priority.cmp(&self.priority))
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

impl PartialOrd for ScheduledTick {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The tick scheduler manages pending block and fluid ticks.
///
/// Uses a priority queue (binary heap) for efficient retrieval of
/// due ticks and a hash set to prevent duplicate scheduling.
pub struct TickScheduler {
    /// Priority queue of pending ticks.
    pending: BinaryHeap<ScheduledTick>,
    /// Set of (pos, tick_type) pairs to prevent duplicates.
    scheduled: HashSet<(BlockPos, TickType)>,
    /// Sequence counter for tie-breaking.
    next_sequence: u64,
}

impl TickScheduler {
    /// Creates a new tick scheduler.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pending: BinaryHeap::new(),
            scheduled: HashSet::new(),
            next_sequence: 0,
        }
    }

    /// Schedules a tick at the given position.
    ///
    /// If a tick is already scheduled for this position and type, it is ignored.
    ///
    /// # Arguments
    /// * `pos` - The block position
    /// * `tick_type` - Block or Fluid tick
    /// * `current_tick` - Current game tick
    /// * `delay` - Number of ticks to wait before triggering
    /// * `priority` - Priority (lower = sooner for same trigger_tick)
    pub fn schedule(
        &mut self,
        pos: BlockPos,
        tick_type: TickType,
        current_tick: u64,
        delay: u32,
        priority: i32,
    ) {
        let key = (pos, tick_type);
        
        // Don't schedule if already pending
        if self.scheduled.contains(&key) {
            return;
        }

        let trigger_tick = current_tick + u64::from(delay);
        let tick = ScheduledTick::new(pos, tick_type, trigger_tick, priority, self.next_sequence);
        self.next_sequence = self.next_sequence.wrapping_add(1);

        self.pending.push(tick);
        self.scheduled.insert(key);

        log::trace!(
            "Scheduled {:?} tick at {:?} for tick {} (delay={})",
            tick_type, pos, trigger_tick, delay
        );
    }

    /// Schedules a fluid tick with default priority (0).
    pub fn schedule_fluid(&mut self, pos: BlockPos, current_tick: u64, delay: u32) {
        self.schedule(pos, TickType::Fluid, current_tick, delay, 0);
    }

    /// Schedules a block tick with default priority (0).
    pub fn schedule_block(&mut self, pos: BlockPos, current_tick: u64, delay: u32) {
        self.schedule(pos, TickType::Block, current_tick, delay, 0);
    }

    /// Returns all ticks that are due at or before the given game tick.
    ///
    /// This removes the ticks from the scheduler.
    pub fn get_due_ticks(&mut self, current_tick: u64) -> Vec<ScheduledTick> {
        let mut due = Vec::new();
        
        // Log pending count occasionally if needed, or if non-zero
        if !self.pending.is_empty() {
            log::trace!("Checking due ticks for current={}, pending count={}", current_tick, self.pending.len());
        }

        while let Some(tick) = self.pending.peek() {
            if tick.trigger_tick > current_tick {
                break;
            }

            let tick = self.pending.pop().expect("peek succeeded");
            self.scheduled.remove(&(tick.pos, tick.tick_type));
            due.push(tick);
        }

        due
    }

    /// Returns the number of pending ticks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Returns true if there are no pending ticks.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Checks if a tick is already scheduled for this position and type.
    #[must_use]
    pub fn is_scheduled(&self, pos: BlockPos, tick_type: TickType) -> bool {
        self.scheduled.contains(&(pos, tick_type))
    }

    /// Clears all pending ticks (used when unloading chunks).
    pub fn clear(&mut self) {
        self.pending.clear();
        self.scheduled.clear();
    }
}

impl Default for TickScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use steel_utils::math::Vector3;

    #[test]
    fn test_schedule_and_retrieve() {
        let mut scheduler = TickScheduler::new();
        let pos1 = BlockPos(Vector3::new(0, 0, 0));
        let pos2 = BlockPos(Vector3::new(1, 0, 0));

        scheduler.schedule_fluid(pos1, 100, 5);
        scheduler.schedule_fluid(pos2, 100, 3);

        assert_eq!(scheduler.len(), 2);

        // At tick 102, nothing is due
        let due = scheduler.get_due_ticks(102);
        assert!(due.is_empty());

        // At tick 103, pos2 is due (delay 3)
        let due = scheduler.get_due_ticks(103);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].pos, pos2);

        // At tick 105, pos1 is due (delay 5)
        let due = scheduler.get_due_ticks(105);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].pos, pos1);

        assert!(scheduler.is_empty());
    }

    #[test]
    fn test_no_duplicates() {
        let mut scheduler = TickScheduler::new();
        let pos = BlockPos(Vector3::new(0, 0, 0));

        scheduler.schedule_fluid(pos, 100, 5);
        scheduler.schedule_fluid(pos, 100, 10); // Should be ignored

        assert_eq!(scheduler.len(), 1);
    }
}
