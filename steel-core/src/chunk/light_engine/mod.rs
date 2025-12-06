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
//! - `ThreadedLevelLightEngine`: Multi-threaded task-based light engine
//!
//! # Current Status
//!
//! This is a **scaffold implementation**. The core structures are in place, but
//! the actual light propagation logic is stubbed out. Future work includes:
//!
//! - Implementing `propagate_increases` and `propagate_decreases` in `LightEngine`
//! - Adding section status tracking (`update_section_status`)
//! - Implementing `set_light_enabled` and `retain_data` flags
//! - Adding chunk neighbor access for cross-chunk propagation
//! - Implementing shape occlusion checking
//! - Adding block light and sky light specific engines
//! - Implementing `propagate_light_sources` for both types

mod base;
pub mod direction;
pub mod light_queue;
pub mod queue_entry;
pub mod threaded_level_light_engine;

// Re-export main types for convenience
pub use base::LightEngine;
pub use direction::Direction;
pub use light_queue::LightQueue;
pub use queue_entry::QueueEntry;
pub use threaded_level_light_engine::{TaskType, ThreadedLevelLightEngine};
