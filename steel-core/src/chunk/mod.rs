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
pub mod chunk_tracker;
/// Tracks the chunks that are visible to a player.
pub mod chunk_tracking_view;
/// Manages chunk distance and ticket system.
pub mod distance_manager;
pub mod flat_chunk_generator;
pub mod level_chunk;
pub mod paletted_container;
pub mod proto_chunk;
pub mod section;
/// Ticket system for chunk loading.
pub mod ticket;
pub mod world_gen_context;
