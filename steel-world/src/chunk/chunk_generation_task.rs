use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
};

use steel_utils::ChunkPos;

use crate::chunk::{chunk_access::ChunkStatus, chunk_holder::ChunkHolder};

pub struct ChunkGenerationTask {
    pub pos: ChunkPos,
    pub target_status: ChunkStatus,
    pub scheduled_status: Option<ChunkStatus>,
    pub marked_for_cancel: AtomicBool,

    pub neighbor_ready: Vec<Box<dyn Future<Output = ()> + Send>>,
    //TODO: We should make a custom struct in the future that can treat this as a fixed size array.
    pub cache: HashMap<ChunkPos, Arc<ChunkHolder>>,
    pub needs_generation: bool,
}

impl ChunkGenerationTask {
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
