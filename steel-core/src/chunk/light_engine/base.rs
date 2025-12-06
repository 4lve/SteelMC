//! Base light engine for flood-fill light propagation.
//!
//! This is a scaffold for the future light propagation implementation.
//! The actual propagation logic (`propagate_increases`, `propagate_decreases`) will be
//! implemented later.

use steel_utils::BlockPos;

use super::{light_queue::LightQueue, queue_entry::QueueEntry};

/// Base light engine that handles light propagation using a flood-fill algorithm.
///
/// This structure maintains two FIFO queues for light propagation:
/// - `increase_queue`: Processes light additions/increases
/// - `decrease_queue`: Processes light removals/decreases
///
/// The light engine follows this execution order:
/// 1. Process all decrease operations first (remove old light)
/// 2. Process all increase operations second (add new light)
///
/// This ordering ensures correct light values at boundaries.
#[derive(Debug)]
pub struct LightEngine {
    /// Queue for light increase operations.
    increase_queue: LightQueue,
    /// Queue for light decrease operations.
    decrease_queue: LightQueue,
}

impl LightEngine {
    /// Creates a new light engine with empty queues.
    #[must_use]
    pub fn new() -> Self {
        Self {
            increase_queue: LightQueue::new(),
            decrease_queue: LightQueue::new(),
        }
    }

    /// Enqueues a light increase at the given position.
    pub fn enqueue_increase(&mut self, pos: BlockPos, entry: QueueEntry) {
        self.increase_queue.enqueue(pos, entry);
    }

    /// Enqueues a light decrease at the given position.
    pub fn enqueue_decrease(&mut self, pos: BlockPos, entry: QueueEntry) {
        self.decrease_queue.enqueue(pos, entry);
    }

    /// Runs all queued light updates.
    ///
    /// This method:
    /// 1. Processes all decreases first (`propagate_decreases`)
    /// 2. Processes all increases second (`propagate_increases`)
    ///
    /// # Note
    /// This is currently a stub. The actual propagation logic will be implemented later.
    pub fn run_light_updates(&mut self) {
        // TODO: Implement actual light propagation
        // For now, just clear the queues to prevent unbounded growth
        self.propagate_decreases();
        self.propagate_increases();
    }

    /// Processes all light decrease operations.
    ///
    /// # Note
    /// This is currently a stub. The actual implementation will:
    /// - Dequeue each entry from `decrease_queue`
    /// - Remove light from the position
    /// - Propagate the decrease to neighbors
    /// - Re-add light sources that still exist
    fn propagate_decreases(&mut self) {
        // TODO: Implement decrease propagation algorithm
        // Stub: just clear the queue
        self.decrease_queue.clear();
    }

    /// Processes all light increase operations.
    ///
    /// # Note
    /// This is currently a stub. The actual implementation will:
    /// - Dequeue each entry from `increase_queue`
    /// - Set light at the position (if from emission)
    /// - Propagate light to neighbors
    /// - Check shape occlusion
    /// - Reduce light by opacity
    fn propagate_increases(&mut self) {
        // TODO: Implement increase propagation algorithm
        // Stub: just clear the queue
        self.increase_queue.clear();
    }

    /// Checks if there are any pending light updates.
    #[must_use]
    pub fn has_work(&self) -> bool {
        !self.increase_queue.is_empty() || !self.decrease_queue.is_empty()
    }

    /// Returns the number of queued increase operations.
    #[must_use]
    pub fn increase_queue_size(&self) -> usize {
        self.increase_queue.len()
    }

    /// Returns the number of queued decrease operations.
    #[must_use]
    pub fn decrease_queue_size(&self) -> usize {
        self.decrease_queue.len()
    }
}

impl Default for LightEngine {
    fn default() -> Self {
        Self::new()
    }
}
