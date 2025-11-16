//! This module contains the `ChunkGenerationTask` struct, which is used to generate chunks.
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use steel_utils::{ChunkPos, math::Vector2};

use crate::chunk::{chunk_access::ChunkStatus, chunk_holder::ChunkHolder};

/// A future that will be ready when the neighbor is ready.
pub type NeighborReady = Pin<Box<dyn Future<Output = Option<()>> + Send + Sync>>;

/// A task that generates a chunk.
pub struct ChunkGenerationTask {
    /// The position of the chunk.
    pub pos: ChunkPos,
    /// The target status of the chunk.
    pub target_status: ChunkStatus,
    /// The status that the chunk is scheduled to be generated to. It's safe to use sync locks cause the it should never be congested with run only running once
    pub scheduled_status: parking_lot::Mutex<Option<ChunkStatus>>,
    /// Whether the task is marked for cancellation.
    pub marked_for_cancel: AtomicBool,

    /// A list of futures that will be ready when the neighbors are ready. It's safe to use sync locks cause the it should never be congested with run only running once
    pub neighbor_ready: parking_lot::Mutex<Vec<NeighborReady>>,
    //TODO: We should make a custom struct in the future that can treat this as a fixed size array.
    /// A cache of chunks that are needed for generation.
    pub cache: HashMap<ChunkPos, Arc<ChunkHolder>>,
    /// Whether the chunk needs to be generated.
    pub needs_generation: AtomicBool,
}

impl ChunkGenerationTask {
    /// Creates a new chunk generation task.
    #[must_use]
    pub fn new(pos: ChunkPos, target_status: ChunkStatus) -> Self {
        Self {
            pos,
            target_status,
            scheduled_status: parking_lot::Mutex::new(None),
            marked_for_cancel: AtomicBool::new(false),
            neighbor_ready: parking_lot::Mutex::new(Vec::new()),
            cache: HashMap::new(),
            needs_generation: AtomicBool::new(true),
        }
    }

    /// Marks the task for cancellation.
    pub fn mark_for_cancel(&self) {
        self.marked_for_cancel.store(true, Ordering::Relaxed);
    }

    /// Starts tasks for neighbors.
    pub fn schedule_layer(&self, _status: ChunkStatus, _needs_generation: bool) {
        let radius = 1; //TODO: this.getRadiusForLayer(status, needsGeneration)
        for x in self.pos.0.x - radius..=self.pos.0.x + radius {
            for y in self.pos.0.y - radius..=self.pos.0.y + radius {
                let _chunk_holder = self.cache.get(&ChunkPos(Vector2::new(x, y)));
                //TODO: scheduleChunkInLayer
            }
        }
    }

    /// Schedules the next layer of the chunk generation. aka dependency layers
    pub fn schedule_next_layer(&self) {
        let status_to_schedule;
        if self.scheduled_status.lock().is_none() {
            status_to_schedule = ChunkStatus::Empty;
            //TODO: canLoadWithoutGeneration()
        } else if !self.needs_generation.load(Ordering::Relaxed)
            && *self.scheduled_status.lock() == Some(ChunkStatus::Empty)
        {
            self.needs_generation.store(true, Ordering::Relaxed);
            status_to_schedule = ChunkStatus::Empty;
        } else {
            // We checked if it was empty above
            status_to_schedule = self.scheduled_status.lock().unwrap().next();
        }

        self.schedule_layer(
            status_to_schedule,
            self.needs_generation.load(Ordering::Relaxed),
        );
        self.scheduled_status.lock().replace(status_to_schedule);
    }

    /// Runs the generation task.
    pub async fn run(self: Arc<Self>) {
        log::info!("Running generation task for chunk {:?}", self.pos);
        self.wait_for_scheduled_layers().await;

        if self.marked_for_cancel.load(Ordering::Relaxed)
            || self.scheduled_status.lock().unwrap_or(ChunkStatus::Empty) == self.target_status
        {
        }
    }

    /// Waits for the scheduled layers to be ready.
    pub async fn wait_for_scheduled_layers(&self) {
        loop {
            let future = {
                let mut guard = self.neighbor_ready.lock();
                guard.pop()
            };

            if let Some(future) = future {
                if future.await.is_none() {
                    self.mark_for_cancel();
                }
            } else {
                break;
            }
        }
    }
}
