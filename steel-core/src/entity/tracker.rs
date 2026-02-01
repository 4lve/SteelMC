//! Entity tracking system for managing which players can see which entities.
//!
//! Uses a chunk-based spatial index similar to `PlayerAreaMap` for efficient
//! tracking. When a player's view changes, we only check entities in the
//! added/removed chunks rather than iterating all entities.
//!
//! The key difference from player tracking is that entities have varying
//! tracking ranges (from their `EntityType`), so we register entities in
//! all chunks within their tracking range.

use std::sync::Arc;

use rustc_hash::FxHashSet;
use steel_protocol::packets::game::{CAddEntity, CRemoveEntities, CSetEntityData};
use steel_registry::REGISTRY;
use steel_utils::ChunkPos;
use steel_utils::locks::SyncRwLock;

use crate::chunk::player_chunk_view::PlayerChunkView;
use crate::entity::{SharedEntity, WeakEntity};
use crate::player::Player;

/// World-level entity tracker using chunk-based spatial indexing.
///
/// Similar to `PlayerAreaMap` but for entities. Maps chunks to entity IDs,
/// allowing O(1) lookup of entities in a chunk when player view changes.
pub struct EntityTracker {
    /// Maps chunk coords to set of entity IDs whose tracking range includes that chunk.
    chunks: scc::HashMap<ChunkPos, FxHashSet<i32>>,

    /// Maps entity ID to its tracking data (weak ref, range, registered chunks, tracking players).
    entities: scc::HashMap<i32, TrackedEntity>,
}

/// Tracking data for a single entity.
struct TrackedEntity {
    /// Weak reference to the entity. When this fails to upgrade, entity is dead.
    entity: WeakEntity,
    /// Tracking range in chunks.
    range_chunks: i32,
    /// Chunks this entity is registered in (for efficient removal).
    registered_chunks: FxHashSet<ChunkPos>,
    /// Players currently tracking this entity (interior mutable for concurrent access).
    seen_by: SyncRwLock<FxHashSet<i32>>,
}

impl Default for EntityTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityTracker {
    /// Creates a new empty entity tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            chunks: scc::HashMap::new(),
            entities: scc::HashMap::new(),
        }
    }

    /// Starts tracking an entity.
    ///
    /// Registers the entity in all chunks within its tracking range.
    pub fn add(&self, entity: &SharedEntity) {
        let entity_id = entity.id();
        let range_chunks = entity.entity_type().client_tracking_range;
        let pos = entity.position();
        let center_chunk = ChunkPos::new((pos.x as i32) >> 4, (pos.z as i32) >> 4);

        // Calculate all chunks within tracking range
        let mut registered_chunks = FxHashSet::default();
        for dx in -range_chunks..=range_chunks {
            for dz in -range_chunks..=range_chunks {
                let chunk = ChunkPos::new(center_chunk.0.x + dx, center_chunk.0.y + dz);
                registered_chunks.insert(chunk);
                self.add_entity_to_chunk(chunk, entity_id);
            }
        }

        let tracked = TrackedEntity {
            entity: Arc::downgrade(entity),
            range_chunks,
            registered_chunks,
            seen_by: SyncRwLock::new(FxHashSet::default()),
        };

        let _ = self.entities.insert_sync(entity_id, tracked);
    }

    /// Stops tracking an entity and sends despawn to all tracking players.
    pub fn remove(&self, entity_id: i32, get_player: impl Fn(i32) -> Option<Arc<Player>>) {
        if let Some((_, tracked)) = self.entities.remove_sync(&entity_id) {
            // Remove from all chunk indices
            for chunk in &tracked.registered_chunks {
                self.remove_entity_from_chunk(*chunk, entity_id);
            }

            // Send despawn to all tracking players
            for player_id in tracked.seen_by.read().iter() {
                if let Some(player) = get_player(*player_id) {
                    player
                        .connection
                        .send_packet(CRemoveEntities::single(entity_id));
                }
            }
        }
    }

    /// Called when a player's view changes. Handles entity spawning/despawning.
    ///
    /// Only checks entities in the added/removed chunks, not all entities.
    pub fn on_player_view_change(
        &self,
        player: &Player,
        added_chunks: &[ChunkPos],
        removed_chunks: &[ChunkPos],
    ) {
        let player_id = player.id;

        // For removed chunks: check if any entities there should stop being tracked
        for &chunk in removed_chunks {
            let entity_ids: Option<Vec<i32>> = self
                .chunks
                .read_sync(&chunk, |_, set| set.iter().copied().collect());

            if let Some(ids) = entity_ids {
                for entity_id in ids {
                    self.entities.update_sync(&entity_id, |_, tracked| {
                        // Check if player was tracking and if entity is no longer in ANY of player's chunks
                        // For simplicity, we remove tracking - if the entity is in another visible chunk,
                        // the added_chunks pass will re-add it
                        if tracked.seen_by.write().remove(&player_id) {
                            player
                                .connection
                                .send_packet(CRemoveEntities::single(entity_id));
                        }
                    });
                }
            }
        }

        // For added chunks: check if any entities there should start being tracked
        for &chunk in added_chunks {
            let entity_ids: Option<Vec<i32>> = self
                .chunks
                .read_sync(&chunk, |_, set| set.iter().copied().collect());

            if let Some(ids) = entity_ids {
                for entity_id in ids {
                    // Skip self
                    if entity_id == player_id {
                        continue;
                    }

                    self.entities.update_sync(&entity_id, |_, tracked| {
                        // Check if not already tracking
                        let mut seen_by = tracked.seen_by.write();
                        if !seen_by.contains(&player_id) {
                            // Try to get entity to send spawn packet
                            if let Some(entity) = tracked.entity.upgrade() {
                                seen_by.insert(player_id);
                                send_spawn_packets(&entity, player);
                            }
                        }
                    });
                }
            }
        }

        // Clean up dead entities we encountered
        self.cleanup_dead_entities();
    }

    /// Called when a player joins - initializes tracking for all visible entities.
    pub fn on_player_join(&self, player: &Player, view: &PlayerChunkView) {
        let player_id = player.id;

        view.for_each(|chunk| {
            let entity_ids: Option<Vec<i32>> = self
                .chunks
                .read_sync(&chunk, |_, set| set.iter().copied().collect());

            if let Some(ids) = entity_ids {
                for entity_id in ids {
                    // Skip self
                    if entity_id == player_id {
                        continue;
                    }

                    self.entities.update_sync(&entity_id, |_, tracked| {
                        let mut seen_by = tracked.seen_by.write();
                        if !seen_by.contains(&player_id)
                            && let Some(entity) = tracked.entity.upgrade()
                        {
                            seen_by.insert(player_id);
                            send_spawn_packets(&entity, player);
                        }
                    });
                }
            }
        });
    }

    /// Called when a player leaves - removes them from all entity tracking.
    pub fn on_player_leave(&self, player_id: i32) {
        // We need to iterate all entities to remove this player
        // This is acceptable since player leave is infrequent
        let mut dead_entities = Vec::new();

        self.entities.iter_sync(|entity_id, tracked| {
            tracked.seen_by.write().remove(&player_id);
            if tracked.entity.strong_count() == 0 {
                dead_entities.push(*entity_id);
            }
            true // continue iteration
        });

        // Clean up any dead entities we found
        for entity_id in dead_entities {
            if let Some((_, tracked)) = self.entities.remove_sync(&entity_id) {
                for chunk in &tracked.registered_chunks {
                    self.remove_entity_from_chunk(*chunk, entity_id);
                }
            }
        }
    }

    /// Updates an entity's position in the chunk index.
    ///
    /// Call this when an entity moves to a new chunk.
    pub fn on_entity_move(
        &self,
        entity_id: i32,
        old_chunk: ChunkPos,
        new_chunk: ChunkPos,
        get_player: impl Fn(i32) -> Option<Arc<Player>>,
    ) {
        if old_chunk == new_chunk {
            return;
        }

        self.entities.update_sync(&entity_id, |_, tracked| {
            let range = tracked.range_chunks;

            // Calculate old and new chunk sets
            let mut old_chunks = FxHashSet::default();
            let mut new_chunks = FxHashSet::default();

            for dx in -range..=range {
                for dz in -range..=range {
                    old_chunks.insert(ChunkPos::new(old_chunk.0.x + dx, old_chunk.0.y + dz));
                    new_chunks.insert(ChunkPos::new(new_chunk.0.x + dx, new_chunk.0.y + dz));
                }
            }

            // Chunks to remove (in old but not in new)
            for chunk in old_chunks.difference(&new_chunks) {
                self.remove_entity_from_chunk(*chunk, entity_id);
                tracked.registered_chunks.remove(chunk);
            }

            // Chunks to add (in new but not in old)
            for chunk in new_chunks.difference(&old_chunks) {
                self.add_entity_to_chunk(*chunk, entity_id);
                tracked.registered_chunks.insert(*chunk);
            }

            // Update tracking for players - those who can no longer see need despawn
            // For now, we rely on the player view change to handle this
            // TODO: Could optimize by checking which players lost/gained visibility
            let _ = get_player; // Suppress unused warning for now
        });
    }

    /// Cleans up dead entities (from unloaded chunks).
    fn cleanup_dead_entities(&self) {
        let mut dead_entities = Vec::new();

        self.entities.iter_sync(|entity_id, tracked| {
            if tracked.entity.strong_count() == 0 {
                dead_entities.push(*entity_id);
            }
            true // continue iteration
        });

        for entity_id in dead_entities {
            if let Some((_, tracked)) = self.entities.remove_sync(&entity_id) {
                for chunk in &tracked.registered_chunks {
                    self.remove_entity_from_chunk(*chunk, entity_id);
                }
                // Note: We don't send despawn packets here because the players
                // will get updated via on_player_view_change when chunks unload
            }
        }
    }

    /// Gets the number of tracked entities.
    #[must_use]
    pub fn count(&self) -> usize {
        self.entities.len()
    }

    /// Sends spawn packets to a specific player and marks them as tracking.
    ///
    /// Used when an entity is spawned and we need to notify nearby players.
    pub fn send_spawn_to_player(&self, entity: &SharedEntity, player: &Player) {
        let entity_id = entity.id();
        let player_id = player.id;

        self.entities.update_sync(&entity_id, |_, tracked| {
            let mut seen_by = tracked.seen_by.write();
            if !seen_by.contains(&player_id) {
                seen_by.insert(player_id);
                send_spawn_packets(entity, player);
            }
        });
    }

    fn add_entity_to_chunk(&self, chunk: ChunkPos, entity_id: i32) {
        if self
            .chunks
            .update_sync(&chunk, |_, set| {
                set.insert(entity_id);
            })
            .is_none()
        {
            let mut set = FxHashSet::default();
            set.insert(entity_id);
            let _ = self.chunks.insert_sync(chunk, set);
        }
    }

    fn remove_entity_from_chunk(&self, chunk: ChunkPos, entity_id: i32) {
        let should_remove = self
            .chunks
            .update_sync(&chunk, |_, set| {
                set.remove(&entity_id);
                set.is_empty()
            })
            .unwrap_or(false);

        if should_remove {
            let _ = self.chunks.remove_if_sync(&chunk, |set| set.is_empty());
        }
    }
}

/// Sends spawn packets for an entity to a player.
fn send_spawn_packets(entity: &SharedEntity, player: &Player) {
    let pos = entity.position();
    let entity_type_id = *REGISTRY.entity_types.get_id(entity.entity_type()) as i32;

    let spawn_packet = CAddEntity {
        id: entity.id(),
        uuid: entity.uuid(),
        entity_type: entity_type_id,
        x: pos.x,
        y: pos.y,
        z: pos.z,
        x_rot: 0, // TODO: Get from entity
        y_rot: 0, // TODO: Get from entity
        head_y_rot: 0,
        data: 0,
    };

    player.connection.send_packet(spawn_packet);

    // Send entity data if any
    let data = entity.pack_all_entity_data();
    if !data.is_empty() {
        player
            .connection
            .send_packet(CSetEntityData::new(entity.id(), data));
    }
}
