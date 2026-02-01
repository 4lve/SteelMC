//! Per-chunk entity storage.
//!
//! Entities are stored in chunks similar to block entities.
//! The chunk owns the `Arc<dyn Entity>` and is responsible for ticking.

use std::sync::Arc;

use rustc_hash::FxHashMap;
use steel_protocol::packets::game::CSetEntityData;
use steel_utils::ChunkPos;
use steel_utils::locks::SyncRwLock;

use super::SharedEntity;
use crate::world::World;

/// Storage for entities in a chunk.
///
/// This mirrors `BlockEntityStorage` - entities are keyed by their ID
/// and ticked from the chunk's tick method.
pub struct EntityStorage {
    /// Entities in this chunk, keyed by entity ID.
    entities: SyncRwLock<FxHashMap<i32, SharedEntity>>,
}

impl EntityStorage {
    /// Creates a new empty entity storage.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entities: SyncRwLock::new(FxHashMap::default()),
        }
    }

    /// Adds an entity to this chunk's storage.
    pub fn add(&self, entity: SharedEntity) {
        let id = entity.id();
        self.entities.write().insert(id, entity);
    }

    /// Removes an entity from this chunk's storage by ID.
    ///
    /// Returns the entity if it was present.
    pub fn remove(&self, entity_id: i32) -> Option<SharedEntity> {
        self.entities.write().remove(&entity_id)
    }

    /// Gets an entity by ID.
    #[must_use]
    pub fn get(&self, entity_id: i32) -> Option<SharedEntity> {
        self.entities.read().get(&entity_id).cloned()
    }

    /// Returns all entities in this chunk.
    #[must_use]
    pub fn get_all(&self) -> Vec<SharedEntity> {
        self.entities.read().values().cloned().collect()
    }

    /// Returns the number of entities in this chunk.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entities.read().len()
    }

    /// Returns whether there are no entities in this chunk.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entities.read().is_empty()
    }

    /// Ticks all entities in this chunk and broadcasts dirty entity data.
    ///
    /// Called from `LevelChunk::tick()`.
    pub fn tick(&self, world: &Arc<World>, chunk_pos: ChunkPos) {
        // Clone to avoid holding lock during tick
        let entities: Vec<SharedEntity> = self.entities.read().values().cloned().collect();

        for entity in entities {
            if entity.is_removed() {
                continue;
            }

            // Entity-specific tick
            entity.tick();

            // Broadcast dirty entity data (base tick behavior)
            if let Some(dirty_data) = entity.pack_dirty_entity_data() {
                let packet = CSetEntityData::new(entity.id(), dirty_data);
                world.broadcast_to_nearby(chunk_pos, packet, None);
            }
        }

        // Cleanup removed entities
        self.entities.write().retain(|_, e| !e.is_removed());
    }

    /// Clears all entities from storage.
    pub fn clear(&self) {
        self.entities.write().clear();
    }
}

impl Default for EntityStorage {
    fn default() -> Self {
        Self::new()
    }
}
