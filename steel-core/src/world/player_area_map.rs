//! Player-specific `AreaMap` for movement broadcast culling.

use std::sync::Arc;

use steel_registry::REGISTRY;
use steel_utils::ChunkPos;
use uuid::Uuid;

use super::area_map::AreaMap;
use crate::player::Player;

/// Spatial index for player proximity queries.
pub struct PlayerAreaMap {
    inner: AreaMap<Uuid>,
}

impl PlayerAreaMap {
    /// Creates a new player area map.
    #[must_use]
    pub fn new() -> Self {
        let tracking_range = REGISTRY
            .entity_types
            .by_key("minecraft:player")
            .map_or(32, |et| et.client_tracking_range as u8);

        Self {
            inner: AreaMap::new(tracking_range),
        }
    }

    /// Creates a new player area map with a custom tracking range.
    #[must_use]
    pub fn with_tracking_range(range: u8) -> Self {
        Self {
            inner: AreaMap::new(range),
        }
    }

    /// Registers a player at their current position.
    pub fn on_player_join(&self, player: &Arc<Player>) {
        let pos = *player.position.lock();
        let chunk = ChunkPos::new((pos.x as i32) >> 4, (pos.z as i32) >> 4);
        self.inner.add(player.gameprofile.id, chunk);
    }

    /// Removes a player from all tracked chunks.
    pub fn on_player_leave(&self, player: &Arc<Player>) {
        self.inner.remove(&player.gameprofile.id);
    }

    /// Updates a player's tracked chunks after moving.
    pub fn on_player_chunk_change(&self, uuid: Uuid, old_chunk: ChunkPos, new_chunk: ChunkPos) {
        self.inner.move_entity(&uuid, old_chunk, new_chunk);
    }

    /// Gets all players tracking the given chunk.
    #[must_use]
    pub fn get_tracking_players(&self, chunk: ChunkPos) -> Vec<Uuid> {
        self.inner.get_entities_tracking_chunk(chunk)
    }

    /// Gets the tracking range in chunks.
    #[must_use]
    pub fn tracking_range(&self) -> u8 {
        self.inner.tracking_radius()
    }

    /// Returns the number of tracked players.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if no players are tracked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Default for PlayerAreaMap {
    fn default() -> Self {
        Self::new()
    }
}
