//! # Steel Core
//!
//! The core library for the Steel Minecraft server. Handles everything related to the PLAY state.
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::cargo,
    missing_docs,
    clippy::unwrap_used
)]
#![allow(
    clippy::single_call_fn,
    clippy::multiple_inherent_impl,
    clippy::shadow_unrelated,
    clippy::missing_errors_doc,
    clippy::struct_excessive_bools,
    clippy::needless_pass_by_value,
    clippy::cargo_common_metadata
)]
use scc::{HashIndex, Queue};
use std::sync::Arc;
use steel_utils::ChunkPos;

use crate::chunk::{
    chunk_access::ChunkStatus, chunk_generation_task::ChunkGenerationTask,
    chunk_holder::ChunkHolder,
};

pub mod chunk;
pub mod config;
pub mod player;
pub mod server;
pub mod world;

/// The root of all worlds.
pub struct Level {
    /// A map of all the chunks in the level.
    pub chunks: ChunkMap,
}

impl Level {
    /// Creates a new level.
    #[must_use]
    pub fn create() -> Self {
        Self {
            chunks: ChunkMap::new(),
        }
    }
}

/// A map of chunks.
pub struct ChunkMap {
    /// A map of all the chunks in the level.
    pub chunks: HashIndex<ChunkPos, Arc<ChunkHolder>>,
    /// A queue of pending generation tasks.
    pub pending_generation_tasks: Queue<Arc<ChunkGenerationTask>>,
}

impl Default for ChunkMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkMap {
    /// Creates a new chunk map.
    #[must_use]
    pub fn new() -> Self {
        Self {
            chunks: HashIndex::new(),
            pending_generation_tasks: Queue::default(),
        }
    }

    /// Schedules a new generation task for the given position and target status.
    ///
    /// Returns a handle to the task.
    pub fn schedule_generation_task(
        &self,
        target_status: ChunkStatus,
        pos: ChunkPos,
    ) -> Arc<ChunkGenerationTask> {
        let task = Arc::new(ChunkGenerationTask::new(pos, target_status));
        self.pending_generation_tasks.push(task.clone());
        task
    }
}
