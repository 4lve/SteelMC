//! FIFO queue for light propagation entries.
//!
//! Uses a simple ring buffer implementation similar to vanilla Minecraft for optimal performance.

use steel_utils::BlockPos;

use super::queue_entry::QueueEntry;

/// A FIFO queue for light propagation using a ring buffer.
///
/// Stores pairs of (`BlockPos`, `QueueEntry`) for processing light changes.
/// The queue processes entries in order to ensure correct light propagation.
///
/// Implementation uses a simple array-based ring buffer with head/tail pointers,
/// matching vanilla Minecraft's approach for minimal overhead.
#[derive(Debug)]
pub struct LightQueue {
    buffer: Vec<(BlockPos, QueueEntry)>,
    head: usize,
    tail: usize,
    size: usize,
}

impl LightQueue {
    /// Creates a new empty light queue with pre-allocated capacity.
    ///
    /// Pre-allocates space for 4096 entries based on typical light propagation workloads,
    /// which significantly reduces reallocation overhead during flood-fill.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(4096)
    }

    /// Creates a new light queue with the specified capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        // Use power-of-2 capacity for faster modulo operations
        let capacity = capacity.next_power_of_two();
        Self {
            buffer: Vec::with_capacity(capacity),
            head: 0,
            tail: 0,
            size: 0,
        }
    }

    /// Enqueues a position and queue entry for processing.
    #[inline]
    pub fn enqueue(&mut self, pos: BlockPos, entry: QueueEntry) {
        // Grow if needed
        if self.size == self.buffer.capacity() {
            self.grow();
        }

        // Add to buffer
        if self.tail < self.buffer.len() {
            self.buffer[self.tail] = (pos, entry);
        } else {
            self.buffer.push((pos, entry));
        }

        self.tail = (self.tail + 1) & (self.buffer.capacity() - 1);
        self.size += 1;
    }

    /// Dequeues the next position and queue entry.
    ///
    /// Returns `None` if the queue is empty.
    #[inline]
    pub fn dequeue(&mut self) -> Option<(BlockPos, QueueEntry)> {
        if self.size == 0 {
            return None;
        }

        let item = self.buffer[self.head];
        self.head = (self.head + 1) & (self.buffer.capacity() - 1);
        self.size -= 1;

        Some(item)
    }

    /// Checks if the queue is empty.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Returns the number of entries in the queue.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    /// Clears all entries from the queue.
    #[inline]
    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.size = 0;
    }

    /// Grows the internal buffer when capacity is reached.
    fn grow(&mut self) {
        let old_capacity = self.buffer.capacity();
        let new_capacity = (old_capacity * 2).max(16);

        let mut new_buffer = Vec::with_capacity(new_capacity);

        // Copy existing elements in order
        for _ in 0..self.size {
            new_buffer.push(self.buffer[self.head]);
            self.head = (self.head + 1) & (old_capacity - 1);
        }

        self.buffer = new_buffer;
        self.head = 0;
        self.tail = self.size;
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
