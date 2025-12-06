//! Light engine for chunk lighting.
//!
//! This module implements Minecraft's light propagation system using a flood-fill
//! algorithm with two FIFO queues for increases and decreases.
//!
//! # Architecture
//!
//! - `Direction`: Cardinal directions for light propagation
//! - `QueueEntry`: Bit-packed u64 encoding light level and propagation metadata
//! - `LightQueue`: FIFO queue for (BlockPos, QueueEntry) pairs
//! - `LightEngine`: Base flood-fill light propagation engine
//! - `LightChunkAccess`: Trait for accessing chunk light and block data
//! - `ThreadedLevelLightEngine`: Multi-threaded task-based light engine
//!
//! # Implementation Status
//!
//! **Completed:**
//! - ✅ Core flood-fill propagation algorithm (`propagate_increases`, `propagate_decreases`)
//! - ✅ Light property getters (luminance, opacity)
//! - ✅ Shape occlusion stub (uses `can_occlude` as approximation)
//!
//! **TODO:**
//! - Adding section status tracking (`update_section_status`)
//! - Implementing `set_light_enabled` and `retain_data` flags
//! - Adding full chunk neighbor access for cross-chunk propagation
//! - Implementing proper VoxelShape face occlusion checking
//! - Adding block light and sky light specific engines
//! - Implementing `propagate_light_sources` for both types

mod base;
pub mod direction;
pub mod light_queue;
pub mod queue_entry;
pub mod threaded_level_light_engine;

// Re-export main types for convenience
pub use base::{LightChunkAccess, LightEngine};
pub use direction::Direction;
pub use light_queue::LightQueue;
pub use queue_entry::QueueEntry;
pub use threaded_level_light_engine::{TaskType, ThreadedLevelLightEngine};
