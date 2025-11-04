use std::sync::Arc;

use scc::{HashIndex, HashMap};
use steel_utils::{ChunkPos, locks::SteelRwLock};

use crate::{ChunkData, player::Player};

mod world_entities;

pub struct World {
    pub loaded_chunks: HashIndex<ChunkPos, SteelRwLock<ChunkData>>,
    pub players: HashMap<uuid::Uuid, Arc<Player>>,
}

impl World {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            loaded_chunks: HashIndex::new(),
            players: HashMap::new(),
        }
    }
}
