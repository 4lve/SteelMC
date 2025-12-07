//! Base light engine for flood-fill light propagation.
//!
//! Implements the core light propagation algorithm based on vanilla Minecraft's approach.
//! Uses a flood-fill algorithm with two priority queues for increases and decreases.

use steel_registry::vanilla_blocks;
use steel_utils::{BlockPos, BlockStateId};

use super::{direction::Direction, light_queue::LightQueue, queue_entry::QueueEntry};

/// Trait providing access to chunk light and block data for the light engine.
///
/// This abstracts away the complexity of chunk storage and cross-chunk access.
#[warn(async_fn_in_trait)]
pub trait LightChunkAccess {
    /// Gets the light level at the given world block position.
    async fn get_light(&self, pos: BlockPos) -> u8;

    /// Sets the light level at the given world block position.
    async fn set_light(&mut self, pos: BlockPos, level: u8) -> ();

    /// Gets the block state at the given world block position.
    async fn get_block_state(&self, pos: BlockPos) -> BlockStateId;

    /// Checks if the block at the given position has an empty collision shape.
    async fn is_empty_shape(&self, pos: BlockPos) -> bool;
}

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
///
/// # Note on Architecture
///
/// This is a simplified implementation that currently only supports within-chunk propagation.
/// Cross-chunk propagation will be added once the chunk access architecture is finalized.
#[derive(Debug)]
pub struct LightEngine {
    /// Queue for light increase operations.
    increase_queue: LightQueue,
    /// Queue for light decrease operations.
    decrease_queue: LightQueue,
}

/// Checks if two block shapes occlude light between them in the given direction.
///
/// This is a STUB implementation that always returns `false` (light passes through).
/// The full implementation would check `VoxelShape` face occlusion.
///
/// # Arguments
/// * `_from_state` - The block state light is coming from
/// * `_to_state` - The block state light is going to
/// * `_direction` - The direction of light propagation
///
/// # Returns
/// `true` if shapes occlude and light cannot pass, `false` otherwise.
///
/// # Note
/// This stub implementation allows all light to pass through.
/// TODO: Implement proper VoxelShape face occlusion using `Shapes::faceShapeOccludes`.
fn shape_occludes(_from_state: BlockStateId, _to_state: BlockStateId, _direction: Direction) -> bool {
    // STUB: Always allow light to pass
    // The real implementation would:
    // 1. Get VoxelShapes for both blocks
    // 2. Get the face shapes in the given direction
    // 3. Check if faces occlude each other using Shapes::faceShapeOccludes
    //
    // For example, stairs and slabs should allow light through gaps
    false
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

    /// Runs all queued light updates with access to chunk data.
    ///
    /// This method:
    /// 1. Processes all decreases first (`propagate_decreases`)
    /// 2. Processes all increases second (`propagate_increases`)
    ///
    /// # Arguments
    /// * `chunk_access` - Provides access to light storage and block states
    pub async fn run_light_updates_with_access<T: LightChunkAccess>(&mut self, chunk_access: &mut T) {
        self.propagate_decreases(chunk_access).await;
        self.propagate_increases(chunk_access).await;
    }

    /// Runs all queued light updates (stub version without chunk access).
    ///
    /// This is a compatibility method that just clears queues.
    /// Use `run_light_updates_with_access` for actual light propagation.
    pub fn run_light_updates(&mut self) {
        // Stub: Just clear queues
        self.decrease_queue.clear();
        self.increase_queue.clear();
    }

    /// Processes all light decrease operations.
    ///
    /// This implements the vanilla flood-fill algorithm for removing light:
    /// 1. Dequeue each (pos, entry) from decrease_queue
    /// 2. For each neighbor in the direction flags:
    ///    - If neighbor's light <= entry.level - 1: propagate decrease
    ///    - Otherwise: re-queue neighbor for increase (it's a light source)
    async fn propagate_decreases<T: LightChunkAccess>(&mut self, chunk_access: &mut T) {
        const ALL_DIRECTIONS: [Direction; 6] = [
            Direction::Down,
            Direction::Up,
            Direction::North,
            Direction::South,
            Direction::West,
            Direction::East,
        ];

        while let Some((pos, entry)) = self.decrease_queue.dequeue() {
            let from_level = entry.level();

            for direction in ALL_DIRECTIONS {
                if !entry.should_propagate(direction) {
                    continue;
                }

                let neighbor_pos = direction.relative(pos);
                let neighbor_light = chunk_access.get_light(neighbor_pos).await;

                if neighbor_light == 0 {
                    continue; // Already dark
                }

                if neighbor_light <= from_level.saturating_sub(1) {
                    // This neighbor's light came from us, remove it
                    chunk_access.set_light(neighbor_pos, 0).await;
                    self.enqueue_decrease(
                        neighbor_pos,
                        QueueEntry::decrease_all_directions(neighbor_light),
                    );
                } else {
                    // This neighbor has its own light source, re-light it
                    self.enqueue_increase(
                        neighbor_pos,
                        QueueEntry::increase_skip_one_direction(
                            neighbor_light,
                            chunk_access.is_empty_shape(neighbor_pos).await,
                            direction.opposite(),
                        ),
                    );
                }
            }
        }
    }

    /// Processes all light increase operations.
    ///
    /// This implements the vanilla flood-fill algorithm for adding light:
    /// 1. Dequeue each (pos, entry) from increase_queue
    /// 2. For each neighbor in the direction flags:
    ///    - Calculate new_light = current_light - max(1, opacity)
    ///    - If new_light > neighbor's light && !shape_occludes:
    ///      - Set neighbor's light to new_light
    ///      - Enqueue neighbor for propagation
    async fn propagate_increases<T: LightChunkAccess>(&mut self, chunk_access: &mut T) {
        const ALL_DIRECTIONS: [Direction; 6] = [
            Direction::Down,
            Direction::Up,
            Direction::North,
            Direction::South,
            Direction::West,
            Direction::East,
        ];

        while let Some((pos, entry)) = self.increase_queue.dequeue() {
            let current_light = chunk_access.get_light(pos).await;

            // Only propagate if light level matches (prevents duplicate processing)
            if current_light != entry.level() {
                continue;
            }

            for direction in ALL_DIRECTIONS {
                if !entry.should_propagate(direction) {
                    continue;
                }

                let neighbor_pos = direction.relative(pos);
                let neighbor_block = chunk_access.get_block_state(neighbor_pos).await;

                // Check shape occlusion between blocks
                let pos_block = chunk_access.get_block_state(pos).await;
                if shape_occludes(pos_block, neighbor_block, direction) {
                    continue;
                }

                // Calculate light reduction (minimum 1, or block's opacity)
                let opacity = vanilla_blocks::get_block_opacity(neighbor_block);
                let reduction = opacity.max(1);

                let new_light = current_light.saturating_sub(reduction);
                let neighbor_light = chunk_access.get_light(neighbor_pos).await;

                if new_light > neighbor_light {
                    chunk_access.set_light(neighbor_pos, new_light).await;
                    self.enqueue_increase(
                        neighbor_pos,
                        QueueEntry::increase_skip_one_direction(
                            new_light,
                            chunk_access.is_empty_shape(neighbor_pos).await,
                            direction.opposite(),
                        ),
                    );
                }
            }
        }
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
