//! This module contains the `ChunkMap` struct, which is a map of chunk holders.
use std::sync::Arc;
use steel_utils::ChunkPos;
use tokio::sync::Mutex;
use tokio_util::task::TaskTracker;

use crate::chunk::chunk_holder::ChunkHolder;
use crate::chunk::{chunk_access::ChunkStatus, chunk_generation_task::ChunkGenerationTask};

/// A map of chunks.
pub struct ChunkMap {
    /// A map of all the chunks in the level.
    pub chunks: scc::HashMap<ChunkPos, Arc<ChunkHolder>>,
    /// A queue of pending generation tasks.
    pub pending_generation_tasks: Mutex<Vec<Arc<ChunkGenerationTask>>>,
    /// A tracker for the generation tasks.
    pub task_tracker: TaskTracker,
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
            chunks: scc::HashMap::new(),
            pending_generation_tasks: Mutex::new(Vec::new()),
            task_tracker: TaskTracker::new(),
        }
    }

    /// Schedules a new generation task for the given position and target status.
    ///
    /// Returns a handle to the task.
    pub async fn schedule_generation_task(
        &self,
        target_status: ChunkStatus,
        pos: ChunkPos,
    ) -> Arc<ChunkGenerationTask> {
        let task = Arc::new(ChunkGenerationTask::new(pos, target_status));
        self.pending_generation_tasks
            .lock()
            .await
            .push(task.clone());
        task
    }

    /// Runs the generation tasks.
    pub async fn run_generation_tasks(&self) {
        let mut pending_generation_tasks = self.pending_generation_tasks.lock().await;
        for task in pending_generation_tasks.drain(..) {
            self.task_tracker.spawn(async move {
                task.run().await;
            });
        }
    }
}
