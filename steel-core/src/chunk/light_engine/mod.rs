//! Minecraft-compatible light propagation system using flood-fill algorithm.

mod base;
mod chunk_cache;
pub mod direction;
pub mod light_queue;
pub mod queue_entry;
mod sky_light_engine;
pub mod threaded_level_light_engine;

// Re-export main types for convenience
pub use base::{BoundaryCrossing, CenterChunkLightAccess, LightChunkAccess, LightEngine};
pub use chunk_cache::ChunkCache;
pub use direction::Direction;
pub use light_queue::LightQueue;
pub use queue_entry::QueueEntry;
pub use sky_light_engine::SkyLightEngine;
pub use threaded_level_light_engine::ThreadedLevelLightEngine;
