//! This module contains all the chunk related structures and logic.

pub mod chunk_access;
pub mod chunk_generation_task;
pub mod chunk_generator;
pub mod chunk_holder;
/// The chunk map manages chunk loading, generation, and lifecycle.
pub mod chunk_map;
pub mod chunk_pyramid;
pub mod chunk_status_tasks;
/// Tracks chunk levels based on ticket propagation.
pub mod chunk_ticket_manager;
/// Tracks the chunks that are visible to a player.
pub mod player_chunk_view;

/// Generates flat worlds with configurable layers.
pub mod flat_chunk_generator;
pub mod level_chunk;
/// Light engine for chunk lighting using flood-fill propagation.
pub mod light_engine;
pub mod light_storage;
pub mod paletted_container;
pub mod proto_chunk;
pub mod section;
pub mod sky_light_sources;

pub mod world_gen_context;

pub use sky_light_sources::ChunkSkyLightSources;
