//! FIFO queue for light propagation entries.

use std::collections::VecDeque;

use steel_utils::BlockPos;

use super::queue_entry::QueueEntry;

/// A FIFO queue for light propagation.
///
/// Stores pairs of (`BlockPos`, `QueueEntry`) for processing light changes.
/// The queue processes entries in order to ensure correct light propagation.
#[derive(Debug)]
pub struct LightQueue {
    queue: VecDeque<(BlockPos, QueueEntry)>,
}

impl LightQueue {
    /// Creates a new empty light queue with pre-allocated capacity.
    ///
    /// Pre-allocates space for 4096 entries based on typical light propagation workloads,
    /// which significantly reduces reallocation overhead during flood-fill.
    #[must_use]
    pub fn new() -> Self {
        Self {
            queue: VecDeque::with_capacity(4096),
        }
    }

    /// Creates a new light queue with the specified capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(capacity),
        }
    }

    /// Enqueues a position and queue entry for processing.
    pub fn enqueue(&mut self, pos: BlockPos, entry: QueueEntry) {
        self.queue.push_back((pos, entry));
    }

    /// Dequeues the next position and queue entry.
    ///
    /// Returns `None` if the queue is empty.
    pub fn dequeue(&mut self) -> Option<(BlockPos, QueueEntry)> {
        self.queue.pop_front()
    }

    /// Checks if the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Returns the number of entries in the queue.
    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Clears all entries from the queue.
    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

impl Default for LightQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use steel_utils::math::Vector3;

    #[test]
    #[allow(clippy::unwrap_used)] // Tests are allowed to panic
    fn test_enqueue_dequeue() {
        let mut queue = LightQueue::new();
        let pos1 = BlockPos(Vector3::new(10, 64, 20));
        let pos2 = BlockPos(Vector3::new(11, 64, 20));
        let entry1 = QueueEntry::decrease_all_directions(5);
        let entry2 = QueueEntry::increase_from_emission(14, true);

        queue.enqueue(pos1, entry1);
        queue.enqueue(pos2, entry2);

        assert_eq!(queue.len(), 2);
        assert!(!queue.is_empty());

        let (dequeued_pos1, dequeued_entry1) = queue.dequeue().unwrap();
        assert_eq!(dequeued_pos1, pos1);
        assert_eq!(dequeued_entry1, entry1);

        let (dequeued_pos2, dequeued_entry2) = queue.dequeue().unwrap();
        assert_eq!(dequeued_pos2, pos2);
        assert_eq!(dequeued_entry2, entry2);

        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_clear() {
        let mut queue = LightQueue::new();
        queue.enqueue(
            BlockPos(Vector3::new(0, 0, 0)),
            QueueEntry::decrease_all_directions(10),
        );
        queue.enqueue(
            BlockPos(Vector3::new(1, 1, 1)),
            QueueEntry::decrease_all_directions(5),
        );

        assert_eq!(queue.len(), 2);
        queue.clear();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }
}
