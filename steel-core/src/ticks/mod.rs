//! Scheduled tick system for blocks and fluids.
//!
//! This module implements Minecraft's scheduled tick system, which allows blocks
//! and fluids to schedule future updates at specific game ticks.
//!
//! # Architecture
//!
//! - [`ScheduledTick`] - A single scheduled tick entry
//! - [`TickPriority`] - Priority for ordering ticks within the same game tick
//! - [`LevelChunkTicks`] - Per-chunk tick storage with deduplication
//! - [`LevelTicks`] - World-level coordinator that manages all chunk ticks

mod chunk_ticks;
mod level_ticks;
mod scheduled_tick;

pub use chunk_ticks::LevelChunkTicks;
pub use level_ticks::LevelTicks;
pub use scheduled_tick::{ScheduledTick, TickKey, TickPriority};
