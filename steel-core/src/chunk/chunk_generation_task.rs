//! This module contains the `ChunkGenerationTask` struct, which is used to generate chunks.
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use futures::future::join_all;
use steel_utils::{ChunkPos, math::Vector2};

use crate::chunk::{
    chunk_access::ChunkStatus,
    chunk_holder::ChunkHolder,
    chunk_map::ChunkMap,
    chunk_pyramid::{GENERATION_PYRAMID, LOADING_PYRAMID},
};

/// A 2D cache that is pre-filled on creation.
/// This is a Rust equivalent of Minecraft's `StaticCache2D`, adapted for `async`.
/// It stores elements in a flat `Vec` for cache efficiency.
pub struct StaticCache2D<T> {
    min_x: i32,
    min_z: i32,
    size: i32,
    /// The cache is stored in a flat Vec in row-major order (Z-then-X).
    cache: Vec<T>,
}

impl<T> StaticCache2D<T> {
    /// Creates a new `StaticCache2D` by calling an async factory function for each position
    /// in the cache's bounds and waiting for all of them to complete concurrently.
    ///
    /// # Arguments
    ///
    /// * `center_x`: The world X coordinate of the center of the cache.
    /// * `center_z`: The world Z coordinate of the center of the cache.
    /// * `radius`: The radius around the center to cache. The total size will be `(radius * 2 + 1)`.
    /// * `factory`: An async function that takes world `(x, z)` coordinates and returns a value to be stored.
    #[allow(clippy::missing_panics_doc)]
    pub async fn create<F, Fut>(center_x: i32, center_z: i32, radius: i32, mut factory: F) -> Self
    where
        F: FnMut(i32, i32) -> Fut,
        Fut: Future<Output = T>,
    {
        let size = radius * 2 + 1;
        let min_x = center_x - radius;
        let min_z = center_z - radius;
        let mut futures = Vec::with_capacity(
            usize::try_from(size * size).expect("Impossible to have a negative size"),
        );

        for z_offset in 0..size {
            for x_offset in 0..size {
                let world_x = x_offset + min_x;
                let world_z = z_offset + min_z;
                futures.push(factory(world_x, world_z));
            }
        }

        let cache = join_all(futures).await;

        Self {
            min_x,
            min_z,
            size,
            cache,
        }
    }

    /// Gets a reference to an element from the cache by world coordinates.
    ///
    /// # Panics
    ///
    /// Panics if the coordinates are out of the cache's bounds.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn get(&self, x: i32, z: i32) -> &T {
        let rel_x = x - self.min_x;
        let rel_z = z - self.min_z;

        if rel_x >= 0 && rel_x < self.size && rel_z >= 0 && rel_z < self.size {
            let index = usize::try_from(rel_z * self.size + rel_x)
                .expect("Impossible to have a negative index");
            &self.cache[index]
        } else {
            panic!(
                "Requested out of range: ({}, {}), bounds are [({}, {}) to ({}, {})]",
                x,
                z,
                self.min_x,
                self.min_z,
                self.min_x + self.size - 1,
                self.min_z + self.size - 1
            );
        }
    }
}

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
    pub cache: Arc<StaticCache2D<Arc<ChunkHolder>>>,
    /// Whether the chunk needs to be generated.
    pub needs_generation: AtomicBool,
}

impl ChunkGenerationTask {
    /// Creates a new chunk generation task.
    #[must_use]
    #[allow(clippy::unwrap_used, clippy::missing_panics_doc)]
    pub async fn new(pos: ChunkPos, target_status: ChunkStatus, chunk_map: Arc<ChunkMap>) -> Self {
        let worst_case_radius = i32::try_from(
            GENERATION_PYRAMID
                .get_step_to(target_status)
                .accumulated_dependencies
                .get_radius_of(ChunkStatus::Empty),
        )
        .unwrap();

        let cache = StaticCache2D::create(pos.0.x, pos.0.y, worst_case_radius, async |x, y| {
            chunk_map
                .chunks
                .get_async(&ChunkPos(Vector2::new(x, y)))
                .await
                .expect("Chunk is required. Should be scheduled at this point")
                .clone()
        })
        .await;

        Self {
            pos,
            target_status,
            scheduled_status: parking_lot::Mutex::new(None),
            marked_for_cancel: AtomicBool::new(false),
            neighbor_ready: parking_lot::Mutex::new(Vec::new()),
            cache: Arc::new(cache),
            needs_generation: AtomicBool::new(true),
        }
    }

    /// Marks the task for cancellation.
    pub fn mark_for_cancel(&self) {
        self.marked_for_cancel.store(true, Ordering::Relaxed);
    }

    /// Schedules a chunk in a layer.
    ///
    /// # Panics
    ///
    /// Panics if the chunk can't be loaded, but didn't expect to need to generate.
    pub fn schedule_chunk_in_layer(
        &self,
        status: ChunkStatus,
        needs_generation: bool,
        chunk_holder: &Arc<ChunkHolder>,
    ) -> bool {
        let persisted_status = chunk_holder.persisted_status();
        let generate = status > persisted_status;
        let pyramid = if generate {
            &*GENERATION_PYRAMID
        } else {
            &*LOADING_PYRAMID
        };

        assert!(
            !generate || needs_generation,
            "Can't load chunk, but didn't expect to need to generate"
        );

        let future = chunk_holder.apply_step(pyramid.get_step_to(status), self.cache.clone());

        self.neighbor_ready.lock().push(future);

        true
    }

    /// Starts tasks for neighbors.
    pub fn schedule_layer(&self, status: ChunkStatus, needs_generation: bool) {
        //TODO: this.getRadiusForLayer(status, needsGeneration)
        let radius = 1;
        for x in (self.pos.0.x - radius)..=(self.pos.0.x + radius) {
            for y in (self.pos.0.y - radius)..=(self.pos.0.y + radius) {
                // When initialized it should always contain it's neighbors
                let chunk_holder = self.cache.get(x, y);
                if self.marked_for_cancel.load(Ordering::Relaxed)
                    || !self.schedule_chunk_in_layer(status, needs_generation, chunk_holder)
                {
                    return;
                }
            }
        }
    }

    /// Schedules the next layer of the chunk generation. aka dependency layers
    ///
    /// # Panics
    ///
    /// Panics if the scheduled status is not the next status of the current status.
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
            status_to_schedule = self
                .scheduled_status
                .lock()
                .expect("Scheduled status is required")
                .next()
                .expect("Next status is required");
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
            return;
        }

        self.schedule_next_layer();
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
