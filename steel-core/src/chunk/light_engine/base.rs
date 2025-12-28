//! Base light engine for flood-fill light propagation.
//!
//! Implements the core light propagation algorithm based on vanilla Minecraft's approach.
//! Uses a flood-fill algorithm with two priority queues for increases and decreases.

use steel_registry::vanilla_blocks;
use steel_utils::{BlockPos, BlockStateId, ChunkPos};

use super::{direction::Direction, light_queue::LightQueue, queue_entry::QueueEntry};

/// A boundary crossing that needs to be processed with cross-chunk access.
#[derive(Debug, Clone, Copy)]
pub struct BoundaryCrossing {
    /// The position in the neighbor chunk.
    pub pos: BlockPos,
    /// The queue entry for propagation.
    pub entry: QueueEntry,
}

/// Trait providing access to chunk light and block data for the light engine.
///
/// This abstracts away the complexity of chunk storage and cross-chunk access.
#[allow(async_fn_in_trait)]
pub trait LightChunkAccess {
    /// Gets the light level at the given world block position.
    async fn get_light(&self, pos: BlockPos) -> u8;

    /// Sets the light level at the given world block position.
    async fn set_light(&mut self, pos: BlockPos, level: u8);

    /// Gets the block state at the given world block position.
    async fn get_block_state(&self, pos: BlockPos) -> BlockStateId;

    /// Checks if the block at the given position has an empty collision shape.
    async fn is_empty_shape(&self, pos: BlockPos) -> bool;
}

/// Synchronous trait for center-chunk-only light access.
///
/// Returns `None` when accessing positions outside the center chunk bounds,
/// allowing the light engine to track boundary crossings for batch processing.
pub trait CenterChunkLightAccess {
    /// Gets the center chunk position.
    fn center_chunk_pos(&self) -> ChunkPos;

    /// Gets the light level at the given position, or None if outside center chunk.
    fn get_light(&self, pos: BlockPos) -> Option<u8>;

    /// Sets the light level at the given position, or returns false if outside center chunk.
    /// Uses interior mutability for light storage.
    fn set_light(&self, pos: BlockPos, level: u8) -> bool;

    /// Gets the block state at the given position, or None if outside center chunk.
    fn get_block_state(&self, pos: BlockPos) -> Option<BlockStateId>;

    /// Checks if the block has an empty collision shape, or None if outside center chunk.
    fn is_empty_shape(&self, pos: BlockPos) -> Option<bool>;

    /// Gets both block state and light level in a single call (optimized).
    ///
    /// This performs coordinate conversion only once, making it more efficient than
    /// calling `get_block_state` and `get_light` separately.
    fn get_neighbor_data(&self, pos: BlockPos) -> Option<(BlockStateId, u8)>;
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
    /// Boundary crossings collected during center-chunk-only propagation.
    boundary_crossings: Vec<BoundaryCrossing>,
}

/// Checks if two block shapes occlude light between them in the given direction.
///
/// Currently a stub that always allows light to pass through.
/// TODO: Implement `VoxelShape` face occlusion checking.
fn shape_occludes(
    _from_state: BlockStateId,
    _to_state: BlockStateId,
    _direction: Direction,
) -> bool {
    false
}

impl LightEngine {
    /// Creates a new light engine with empty queues.
    #[must_use]
    pub fn new() -> Self {
        Self {
            increase_queue: LightQueue::new(),
            decrease_queue: LightQueue::new(),
            boundary_crossings: Vec::new(),
        }
    }

    /// Returns the boundary crossings collected during center-chunk propagation.
    #[must_use]
    pub fn boundary_crossings(&self) -> &[BoundaryCrossing] {
        &self.boundary_crossings
    }

    /// Takes the boundary crossings, leaving the vector empty.
    pub fn take_boundary_crossings(&mut self) -> Vec<BoundaryCrossing> {
        std::mem::take(&mut self.boundary_crossings)
    }

    /// Clears boundary crossings.
    pub fn clear_boundary_crossings(&mut self) {
        self.boundary_crossings.clear();
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
    pub async fn run_light_updates_with_access<T: LightChunkAccess>(
        &mut self,
        chunk_access: &mut T,
    ) {
        self.propagate_decreases(chunk_access).await;
        self.propagate_increases(chunk_access).await;
    }

    /// Runs queued light updates within the center chunk only (synchronous, lock-free).
    ///
    /// This method:
    /// 1. Processes all decreases within center chunk
    /// 2. Processes all increases within center chunk
    /// 3. Collects boundary crossings for batch processing
    ///
    /// # Arguments
    /// * `chunk_access` - Provides synchronous access to center chunk only
    pub fn run_center_chunk_updates<T: CenterChunkLightAccess>(&mut self, chunk_access: &T) {
        self.propagate_decreases_center(chunk_access);
        self.propagate_increases_center(chunk_access);
    }

    /// Processes all light decrease operations.
    ///
    /// This implements the vanilla flood-fill algorithm for removing light:
    /// 1. Dequeue each (pos, entry) from `decrease_queue`
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
    /// 1. Dequeue each (pos, entry) from `increase_queue`
    /// 2. For each neighbor in the direction flags:
    ///    - Calculate `new_light` = `current_light` - max(1, opacity)
    ///    - If `new_light` > neighbor's light && !`shape_occludes`:
    ///      - Set neighbor's light to `new_light`
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

    /// Processes light decreases within the center chunk only (synchronous).
    ///
    /// When encountering positions outside the center chunk, records them as boundary crossings
    /// instead of propagating across chunk boundaries.
    fn propagate_decreases_center<T: CenterChunkLightAccess>(&mut self, chunk_access: &T) {
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

                // Check if neighbor is outside center chunk
                let Some(neighbor_light) = chunk_access.get_light(neighbor_pos) else {
                    // Only create boundary crossing if position is within valid world Y bounds
                    let neighbor_chunk_x = neighbor_pos.0.x >> 4;
                    let neighbor_chunk_z = neighbor_pos.0.z >> 4;
                    let center_chunk_pos = chunk_access.center_chunk_pos();
                    let is_horizontal_boundary = neighbor_chunk_x != center_chunk_pos.0.x
                        || neighbor_chunk_z != center_chunk_pos.0.y;

                    if is_horizontal_boundary && from_level > 0 {
                        self.boundary_crossings.push(BoundaryCrossing {
                            pos: neighbor_pos,
                            entry: QueueEntry::decrease_all_directions(from_level),
                        });
                    }
                    // If it's not a horizontal boundary, it must be out of Y bounds - discard
                    continue;
                };

                if neighbor_light == 0 {
                    continue; // Already dark
                }

                if neighbor_light <= from_level.saturating_sub(1) {
                    // This neighbor's light came from us, remove it
                    if chunk_access.set_light(neighbor_pos, 0) {
                        self.enqueue_decrease(
                            neighbor_pos,
                            QueueEntry::decrease_all_directions(neighbor_light),
                        );
                    }
                } else {
                    // This neighbor has its own light source, re-light it
                    let is_empty = chunk_access.is_empty_shape(neighbor_pos).unwrap_or(true);
                    self.enqueue_increase(
                        neighbor_pos,
                        QueueEntry::increase_skip_one_direction(
                            neighbor_light,
                            is_empty,
                            direction.opposite(),
                        ),
                    );
                }
            }
        }
    }

    /// Processes light increases within the center chunk only (synchronous).
    ///
    /// When encountering positions outside the center chunk, records them as boundary crossings
    /// instead of propagating across chunk boundaries.
    fn propagate_increases_center<T: CenterChunkLightAccess>(&mut self, chunk_access: &T) {
        const ALL_DIRECTIONS: [Direction; 6] = [
            Direction::Down,
            Direction::Up,
            Direction::North,
            Direction::South,
            Direction::West,
            Direction::East,
        ];

        while let Some((pos, entry)) = self.increase_queue.dequeue() {
            // Check if position is in center chunk
            let Some(current_light) = chunk_access.get_light(pos) else {
                // Position outside center chunk - only create boundary crossing if it's
                // a horizontal boundary, not if it's outside Y bounds
                let pos_chunk_x = pos.0.x >> 4;
                let pos_chunk_z = pos.0.z >> 4;
                let center_chunk_pos = chunk_access.center_chunk_pos();
                let is_horizontal_boundary =
                    pos_chunk_x != center_chunk_pos.0.x || pos_chunk_z != center_chunk_pos.0.y;

                if is_horizontal_boundary {
                    self.boundary_crossings
                        .push(BoundaryCrossing { pos, entry });
                }
                // If not horizontal boundary, position is outside Y bounds - discard
                continue;
            };

            // Only propagate if light level matches (prevents duplicate processing)
            if current_light != entry.level() {
                continue;
            }

            // OPTIMIZATION: Cache current block state - used in all 6 direction checks
            let Some(pos_block) = chunk_access.get_block_state(pos) else {
                continue;
            };

            for direction in ALL_DIRECTIONS {
                if !entry.should_propagate(direction) {
                    continue;
                }

                let neighbor_pos = direction.relative(pos);

                // Get neighbor block state and light in a single call (optimized)
                let neighbor_data_opt = chunk_access.get_neighbor_data(neighbor_pos);

                // If neighbor is outside center chunk, defer to Phase 2
                let Some((neighbor_block, neighbor_light)) = neighbor_data_opt else {
                    // Only create boundary crossing if position is within valid world Y bounds
                    // Positions outside Y bounds (above/below world) are discarded
                    let neighbor_chunk_x = neighbor_pos.0.x >> 4;
                    let neighbor_chunk_z = neighbor_pos.0.z >> 4;
                    let center_chunk_pos = chunk_access.center_chunk_pos();
                    let is_horizontal_boundary = neighbor_chunk_x != center_chunk_pos.0.x
                        || neighbor_chunk_z != center_chunk_pos.0.y;

                    if is_horizontal_boundary {
                        // Calculate new light assuming air (opacity 0, reduction 1)
                        let new_light = current_light.saturating_sub(1);
                        if new_light > 0 {
                            self.boundary_crossings.push(BoundaryCrossing {
                                pos: neighbor_pos,
                                entry: QueueEntry::increase_skip_one_direction(
                                    new_light,
                                    true, // Assume empty until Phase 2 checks
                                    direction.opposite(),
                                ),
                            });
                        }
                    }
                    // If it's not a horizontal boundary, it must be out of Y bounds - discard
                    continue;
                };

                // Check shape occlusion between blocks (using cached pos_block)
                if shape_occludes(pos_block, neighbor_block, direction) {
                    continue;
                }

                // Calculate light reduction (minimum 1, or block's opacity)
                let opacity = vanilla_blocks::get_block_opacity(neighbor_block);
                let reduction = opacity.max(1);

                let new_light = current_light.saturating_sub(reduction);

                if new_light > neighbor_light && chunk_access.set_light(neighbor_pos, new_light) {
                    let is_empty = chunk_access.is_empty_shape(neighbor_pos).unwrap_or(true);
                    self.enqueue_increase(
                        neighbor_pos,
                        QueueEntry::increase_skip_one_direction(
                            new_light,
                            is_empty,
                            direction.opposite(),
                        ),
                    );
                }
            }
        }
    }
}

impl Default for LightEngine {
    fn default() -> Self {
        Self::new()
    }
}
