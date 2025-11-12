//! This module contains the `ChunkGenerationTask` struct, which is used to generate chunks.
use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, atomic::AtomicBool},
};

use steel_utils::ChunkPos;

use crate::chunk::{chunk_access::ChunkStatus, chunk_holder::ChunkHolder};

/// A task that generates a chunk.
pub struct ChunkGenerationTask {
    /// The position of the chunk.
    pub pos: ChunkPos,
    /// The target status of the chunk.
    pub target_status: ChunkStatus,
    /// The status that the chunk is scheduled to be generated to.
    pub scheduled_status: Option<ChunkStatus>,
    /// Whether the task is marked for cancellation.
    pub marked_for_cancel: AtomicBool,

    /// A list of futures that will be ready when the neighbors are ready.
    pub neighbor_ready: Vec<Box<dyn Future<Output = ()> + Send>>,
    //TODO: We should make a custom struct in the future that can treat this as a fixed size array.
    /// A cache of chunks that are needed for generation.
    pub cache: HashMap<ChunkPos, Arc<ChunkHolder>>,
    /// Whether the chunk needs to be generated.
    pub needs_generation: bool,
}

impl ChunkGenerationTask {
    /// Creates a new chunk generation task.
    #[must_use]
    pub fn new(pos: ChunkPos, target_status: ChunkStatus) -> Self {
        Self {
            pos,
            target_status,
            scheduled_status: None,
            marked_for_cancel: AtomicBool::new(false),
            neighbor_ready: Vec::new(),
            cache: HashMap::new(),
            needs_generation: true,
        }
    }
}
