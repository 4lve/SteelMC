//! This module contains the `World` struct, which represents a world.
use std::sync::Arc;

use scc::HashMap;
use steel_registry::blocks::BlockRegistry;
use tokio::runtime::Runtime;
use uuid::Uuid;

use crate::{ChunkMap, player::Player};

mod world_entities;

/// A struct that represents a world.
pub struct World {
    /// The chunk map of the world.
    pub chunk_map: Arc<ChunkMap>,
    /// A map of all the players in the world.
    pub players: HashMap<Uuid, Arc<Player>>,
}

impl World {
    /// Creates a new world.
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new(block_registry: &BlockRegistry, chunk_runtime: Arc<Runtime>) -> Self {
        Self {
            chunk_map: Arc::new(ChunkMap::new(block_registry, chunk_runtime)),
            players: HashMap::new(),
        }
    }

    /// Ticks the world.
    pub fn tick_b(&self, tick_count: u64) {
        self.chunk_map.tick_b(tick_count);

        // Tick players
        self.players.iter_sync(|_uuid, player| {
            player.tick();

            true
        });
    }
}
