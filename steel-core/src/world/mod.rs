//! This module contains the `World` struct, which represents a world.
use std::sync::Arc;

use scc::HashMap;
use uuid::Uuid;

use crate::{ChunkMap, player::Player};

mod world_entities;

/// A struct that represents a world.
pub struct World {
    /// The chunk map of the world.
    pub chunk_map: ChunkMap,
    /// A map of all the players in the world.
    pub players: HashMap<Uuid, Arc<Player>>,
}

impl World {
    /// Creates a new world.
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            chunk_map: ChunkMap::new(),
            players: HashMap::new(),
        }
    }
}
