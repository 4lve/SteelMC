use std::sync::Arc;

use scc::HashMap;
use uuid::Uuid;

use crate::{ChunkMap, player::Player};

mod world_entities;

pub struct World {
    pub chunk_map: ChunkMap,
    pub players: HashMap<Uuid, Arc<Player>>,
}

impl World {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            chunk_map: ChunkMap::new(),
            players: HashMap::new(),
        }
    }
}
