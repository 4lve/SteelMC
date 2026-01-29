//! Scheduled tick types.

use std::cmp::Ordering;

use steel_utils::BlockPos;

/// Priority level for scheduled ticks.
///
/// When multiple ticks fire on the same game tick, they are processed
/// in priority order (higher priority first), then by sub-tick order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(i8)]
pub enum TickPriority {
    /// Extremely high priority (-3)
    ExtremelyHigh = -3,
    /// Very high priority (-2)
    VeryHigh = -2,
    /// High priority (-1)
    High = -1,
    /// Normal priority (0) - default
    #[default]
    Normal = 0,
    /// Low priority (1)
    Low = 1,
    /// Very low priority (2)
    VeryLow = 2,
    /// Extremely low priority (3)
    ExtremelyLow = 3,
}

impl TickPriority {
    /// Returns the numeric value of this priority.
    /// Lower values = higher priority.
    #[inline]
    #[must_use]
    pub const fn value(self) -> i8 {
        self as i8
    }
}

impl PartialOrd for TickPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TickPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority (lower numeric value) should be Greater for BinaryHeap
        // So we reverse: High(-1) > Normal(0) > Low(1)
        other.value().cmp(&self.value())
    }
}

/// A scheduled tick for a block or fluid.
///
/// Scheduled ticks are used by blocks like fire, redstone repeaters,
/// and fluids to schedule future updates at specific game ticks.
#[derive(Debug, Clone)]
pub struct ScheduledTick<T> {
    /// The type being ticked (block ID or fluid ID).
    pub tick_type: T,
    /// The position of the block/fluid.
    pub pos: BlockPos,
    /// The absolute game tick when this should fire.
    pub trigger_tick: u64,
    /// Priority for ordering within the same game tick.
    pub priority: TickPriority,
    /// Sub-tick ordering for ticks with same `trigger_tick` and priority.
    /// Lower values run first.
    pub sub_tick_order: u64,
}

impl<T> ScheduledTick<T> {
    /// Creates a new scheduled tick with normal priority.
    pub fn new(tick_type: T, pos: BlockPos, trigger_tick: u64, sub_tick_order: u64) -> Self {
        Self {
            tick_type,
            pos,
            trigger_tick,
            priority: TickPriority::Normal,
            sub_tick_order,
        }
    }

    /// Creates a new scheduled tick with the specified priority.
    pub fn with_priority(
        tick_type: T,
        pos: BlockPos,
        trigger_tick: u64,
        priority: TickPriority,
        sub_tick_order: u64,
    ) -> Self {
        Self {
            tick_type,
            pos,
            trigger_tick,
            priority,
            sub_tick_order,
        }
    }
}

impl<T: PartialEq> PartialEq for ScheduledTick<T> {
    fn eq(&self, other: &Self) -> bool {
        self.trigger_tick == other.trigger_tick
            && self.priority == other.priority
            && self.sub_tick_order == other.sub_tick_order
    }
}

impl<T: Eq> Eq for ScheduledTick<T> {}

impl<T: Eq> PartialOrd for ScheduledTick<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Eq> Ord for ScheduledTick<T> {
    /// Ordering for the priority queue.
    ///
    /// Note: `BinaryHeap` is a max-heap, so we reverse the comparison
    /// to get earliest ticks first.
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by trigger_tick (earlier = higher priority in queue)
        match other.trigger_tick.cmp(&self.trigger_tick) {
            Ordering::Equal => {}
            ord => return ord,
        }
        // Then by priority (lower value = higher priority)
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => {}
            ord => return ord,
        }
        // Finally by sub_tick_order (lower = first)
        other.sub_tick_order.cmp(&self.sub_tick_order)
    }
}

/// Key for deduplication in the tick set.
///
/// Only considers position and type - ignores timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TickKey<T> {
    /// The position of the block/fluid.
    pub pos: BlockPos,
    /// The type being ticked.
    pub tick_type: T,
}

impl<T: Copy> From<&ScheduledTick<T>> for TickKey<T> {
    fn from(tick: &ScheduledTick<T>) -> Self {
        Self {
            pos: tick.pos,
            tick_type: tick.tick_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use steel_utils::math::Vector3;

    #[test]
    fn test_tick_ordering() {
        let pos = BlockPos(Vector3::new(0, 0, 0));

        // Earlier trigger_tick should come first
        let tick1 = ScheduledTick::new(1u32, pos, 100, 0);
        let tick2 = ScheduledTick::new(1u32, pos, 200, 0);
        assert!(tick1 > tick2); // In max-heap, "greater" means higher priority

        // Same trigger_tick, higher priority should come first
        let tick3 = ScheduledTick::with_priority(1u32, pos, 100, TickPriority::High, 0);
        let tick4 = ScheduledTick::with_priority(1u32, pos, 100, TickPriority::Normal, 0);
        assert!(tick3 > tick4);

        // Same trigger_tick and priority, lower sub_tick_order should come first
        let tick5 = ScheduledTick::new(1u32, pos, 100, 5);
        let tick6 = ScheduledTick::new(1u32, pos, 100, 10);
        assert!(tick5 > tick6);
    }
}
