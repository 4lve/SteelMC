//! Entity tracking system
//!
//! Manages which entities are visible to which players based on distance and chunks.

use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;
use uuid::Uuid;

use super::{Entity, TrackedEntity};
use crate::player::Player;
use steel_utils::ChunkPos;
use steel_utils::locks::{SyncMutex, SyncRwLock};

/// Default entity tracking range in blocks
pub const DEFAULT_ENTITY_TRACKING_RANGE: i32 = 64;

/// Entity tracker that manages visibility between entities and players
pub struct EntityTracker {
    /// Map of entity ID to tracked entity
    tracked_entities: SyncRwLock<FxHashMap<i32, Arc<TrackedEntity>>>,

    /// Map of player UUID to their visible entities
    player_tracking: SyncRwLock<FxHashMap<Uuid, FxHashSet<i32>>>,

    /// Global entity ID counter
    next_entity_id: SyncMutex<i32>,
}

impl EntityTracker {
    /// Creates a new entity tracker
    #[must_use]
    pub fn new() -> Self {
        Self {
            tracked_entities: SyncRwLock::new(FxHashMap::default()),
            player_tracking: SyncRwLock::new(FxHashMap::default()),
            next_entity_id: SyncMutex::new(1_000_000),
        }
    }

    /// Allocates a new unique entity ID
    pub fn allocate_entity_id(&self) -> i32 {
        let mut next_id = self.next_entity_id.lock();
        let id = *next_id;
        *next_id += 1;
        id
    }

    /// Adds an entity to tracking
    pub fn add_entity(&self, entity: Arc<dyn Entity>, tracking_range: Option<i32>) {
        let entity_id = entity.entity_id();
        let range = tracking_range.unwrap_or(DEFAULT_ENTITY_TRACKING_RANGE);

        let tracked = Arc::new(TrackedEntity::new(entity, range));
        self.tracked_entities.write().insert(entity_id, tracked);
    }

    /// Removes an entity from tracking
    pub fn remove_entity(&self, entity_id: i32) {
        if let Some(tracked) = self.tracked_entities.write().remove(&entity_id) {
            // Remove from all players' tracking sets
            tracked.broadcast_removal();

            let mut player_tracking = self.player_tracking.write();
            for visible_set in player_tracking.values_mut() {
                visible_set.remove(&entity_id);
            }
        }
    }

    /// Updates entity visibility for all players
    ///
    /// This should be called every tick to update which entities players can see
    pub fn tick(&self, players: &[Arc<Player>]) {
        let tracked_entities = self.tracked_entities.read();

        for player in players {
            let player_uuid = player.gameprofile.id;
            let player_pos = *player.position.lock();
            let player_chunk = *player.last_chunk_pos.lock();

            // Get or create tracking set for this player
            let mut player_tracking = self.player_tracking.write();
            let visible_entities = player_tracking.entry(player_uuid).or_default();

            // Check each tracked entity
            for tracked in tracked_entities.values() {
                let entity_id = tracked.entity.entity_id();

                // Don't track self
                if tracked.entity.uuid() == player_uuid {
                    continue;
                }

                let entity_pos = tracked.entity.position();

                // Calculate distance
                let dx = player_pos.x - entity_pos.x;
                let dy = player_pos.y - entity_pos.y;
                let dz = player_pos.z - entity_pos.z;
                let distance_squared = dx * dx + dy * dy + dz * dz;

                let tracking_range_squared = f64::from(tracked.tracking_range).powi(2);

                let should_track = distance_squared <= tracking_range_squared
                    && Self::is_chunk_tracked(player, player_chunk, &entity_pos);

                let currently_tracked = visible_entities.contains(&entity_id);

                if should_track && !currently_tracked {
                    // Start tracking
                    visible_entities.insert(entity_id);
                    tracked.add_player(player.clone());
                } else if !should_track && currently_tracked {
                    // Stop tracking
                    visible_entities.remove(&entity_id);
                    tracked.remove_player(player_uuid);
                }
            }
        }

        // Send updates for entities that moved or changed
        for tracked in tracked_entities.values() {
            tracked.send_changes();
        }
    }

    /// Checks if a chunk is being tracked by the player
    fn is_chunk_tracked(
        player: &Player,
        player_chunk: ChunkPos,
        entity_pos: &steel_utils::math::Vector3<f64>,
    ) -> bool {
        #[allow(clippy::cast_possible_truncation)]
        let entity_chunk = ChunkPos::new((entity_pos.x as i32) >> 4, (entity_pos.z as i32) >> 4);

        // Get player's chunk view
        let tracking_view = player.last_tracking_view.lock();
        if let Some(view) = tracking_view.as_ref() {
            view.contains(entity_chunk)
        } else {
            // If no tracking view yet, just check if same chunk
            player_chunk == entity_chunk
        }
    }

    /// Gets a tracked entity by ID
    pub fn get_entity(&self, entity_id: i32) -> Option<Arc<TrackedEntity>> {
        self.tracked_entities.read().get(&entity_id).cloned()
    }

    /// Removes a player from all entity tracking
    pub fn remove_player(&self, player_uuid: Uuid) {
        // Remove player from all tracked entities
        let tracked_entities = self.tracked_entities.read();
        for tracked in tracked_entities.values() {
            tracked.remove_player(player_uuid);
        }

        // Remove player's tracking set
        self.player_tracking.write().remove(&player_uuid);
    }

    /// Gets the number of tracked entities
    pub fn entity_count(&self) -> usize {
        self.tracked_entities.read().len()
    }

    /// Immediately updates visibility for a specific player (called when player joins)
    /// This matches vanilla's behavior of calling `updatePlayers()` right after adding an entity
    pub fn update_player_visibility(&self, player: &Arc<Player>) {
        let player_uuid = player.gameprofile.id;
        let player_pos = *player.position.lock();

        #[allow(clippy::cast_possible_truncation)]
        let player_chunk = ChunkPos::new((player_pos.x as i32) >> 4, (player_pos.z as i32) >> 4);

        let mut player_tracking = self.player_tracking.write();
        let visible_entities = player_tracking.entry(player_uuid).or_default();

        let tracked_entities = self.tracked_entities.read();

        for tracked in tracked_entities.values() {
            let entity_id = tracked.entity.entity_id();

            // Don't track self
            if tracked.entity.uuid() == player_uuid {
                continue;
            }

            let entity_pos = tracked.entity.position();

            // Calculate distance
            let dx = player_pos.x - entity_pos.x;
            let dy = player_pos.y - entity_pos.y;
            let dz = player_pos.z - entity_pos.z;
            let distance_squared = dx * dx + dy * dy + dz * dz;

            let tracking_range_squared = f64::from(tracked.tracking_range).powi(2);

            let should_track = distance_squared <= tracking_range_squared
                && Self::is_chunk_tracked(player, player_chunk, &entity_pos);

            if should_track {
                visible_entities.insert(entity_id);
                tracked.add_player(player.clone());
            }
        }
    }
}

impl Default for EntityTracker {
    fn default() -> Self {
        Self::new()
    }
}
