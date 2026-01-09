//! Spatial data structure for efficient entity proximity queries.
//!
//! Based on VMP (Very Many Players) implementation pattern:
//! Maps chunk coordinates to sets of entities for O(1) nearby entity lookup.

use rustc_hash::FxHashSet;
use scc::HashMap;
use steel_utils::ChunkPos;

/// A spatial data structure that maps chunk coordinates to sets of entities.
///
/// Uses packed i64 chunk coordinates as keys for efficient hashing.
/// Thread-safe via `scc::HashMap` for concurrent access.
///
/// The map maintains a dual index:
/// - `chunks`: Maps chunk coords to entities whose tracking area includes that chunk
/// - `entity_chunks`: Maps entity IDs to the set of chunks they're registered in
///
/// This enables O(1) lookup of nearby entities and O(tracking area) removal.
pub struct AreaMap<T: Clone + Eq + std::hash::Hash + Send + Sync + 'static> {
    /// Maps packed chunk coords (i64) to set of entity identifiers
    chunks: HashMap<i64, FxHashSet<T>>,

    /// Maps entity ID to its current set of tracked chunks (for efficient removal)
    entity_chunks: HashMap<T, FxHashSet<i64>>,

    /// The tracking radius in chunks
    tracking_radius: u8,
}

impl<T: Clone + Eq + std::hash::Hash + Send + Sync + 'static> AreaMap<T> {
    /// Creates a new `AreaMap` with the given tracking radius.
    #[must_use]
    pub fn new(tracking_radius: u8) -> Self {
        Self {
            chunks: HashMap::new(),
            entity_chunks: HashMap::new(),
            tracking_radius,
        }
    }

    /// Gets the current tracking radius.
    #[must_use]
    pub fn tracking_radius(&self) -> u8 {
        self.tracking_radius
    }

    /// Adds an entity at the given chunk position.
    pub fn add(&self, entity: T, center_chunk: ChunkPos) {
        let tracked_chunks = self.calculate_tracked_chunks(center_chunk);

        let mut entity_set = FxHashSet::default();
        for &chunk_packed in &tracked_chunks {
            entity_set.insert(chunk_packed);
        }
        let _ = self.entity_chunks.insert_sync(entity.clone(), entity_set);

        for chunk_packed in tracked_chunks {
            self.add_to_chunk(chunk_packed, entity.clone());
        }
    }

    /// Removes an entity from all tracked chunks.
    pub fn remove(&self, entity: &T) {
        if let Some((_, chunks)) = self.entity_chunks.remove_sync(entity) {
            for chunk_packed in chunks {
                self.remove_from_chunk(chunk_packed, entity);
            }
        }
    }

    /// Updates an entity's position from old chunk to new chunk.
    pub fn move_entity(&self, entity: &T, old_center: ChunkPos, new_center: ChunkPos) {
        if old_center == new_center {
            return;
        }

        let old_chunks: FxHashSet<i64> = self
            .calculate_tracked_chunks(old_center)
            .into_iter()
            .collect();
        let new_chunks: FxHashSet<i64> = self
            .calculate_tracked_chunks(new_center)
            .into_iter()
            .collect();

        for &chunk_packed in old_chunks.difference(&new_chunks) {
            self.remove_from_chunk(chunk_packed, entity);
        }

        for &chunk_packed in new_chunks.difference(&old_chunks) {
            self.add_to_chunk(chunk_packed, entity.clone());
        }

        let _ = self.entity_chunks.insert_sync(entity.clone(), new_chunks);
    }

    /// Gets all entities that are tracking the given chunk.
    #[must_use]
    pub fn get_entities_tracking_chunk(&self, chunk: ChunkPos) -> Vec<T> {
        let chunk_packed = chunk.as_i64();
        self.chunks
            .read_sync(&chunk_packed, |_, set: &FxHashSet<T>| {
                set.iter().cloned().collect()
            })
            .unwrap_or_default()
    }

    /// Returns the number of tracked entities.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entity_chunks.len()
    }

    /// Returns true if no entities are tracked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entity_chunks.is_empty()
    }

    /// Clears all entities from the map.
    pub fn clear(&self) {
        self.chunks.clear_sync();
        self.entity_chunks.clear_sync();
    }

    /// Calculates all chunks within tracking radius of a center chunk.
    fn calculate_tracked_chunks(&self, center: ChunkPos) -> Vec<i64> {
        let radius = i32::from(self.tracking_radius);
        let mut chunks = Vec::with_capacity(((radius * 2 + 1) * (radius * 2 + 1)) as usize);

        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let chunk = ChunkPos::new(center.0.x + dx, center.0.y + dz);
                chunks.push(chunk.as_i64());
            }
        }

        chunks
    }

    fn add_to_chunk(&self, chunk_packed: i64, entity: T) {
        if self
            .chunks
            .update_sync(&chunk_packed, |_, set: &mut FxHashSet<T>| {
                set.insert(entity.clone());
            })
            .is_none()
        {
            let mut set = FxHashSet::default();
            set.insert(entity);
            let _ = self.chunks.insert_sync(chunk_packed, set);
        }
    }

    fn remove_from_chunk(&self, chunk_packed: i64, entity: &T) {
        let should_remove = self
            .chunks
            .update_sync(&chunk_packed, |_, set: &mut FxHashSet<T>| {
                set.remove(entity);
                set.is_empty()
            })
            .unwrap_or(false);

        if should_remove {
            let _ = self
                .chunks
                .remove_if_sync(&chunk_packed, |set| set.is_empty());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_add_and_get() {
        let map: AreaMap<Uuid> = AreaMap::new(2);
        let entity = Uuid::new_v4();
        let center = ChunkPos::new(0, 0);

        map.add(entity, center);

        assert!(map.get_entities_tracking_chunk(center).contains(&entity));
        assert!(
            map.get_entities_tracking_chunk(ChunkPos::new(1, 1))
                .contains(&entity)
        );
        assert!(
            map.get_entities_tracking_chunk(ChunkPos::new(2, 2))
                .contains(&entity)
        );
        assert!(
            !map.get_entities_tracking_chunk(ChunkPos::new(3, 3))
                .contains(&entity)
        );
    }

    #[test]
    fn test_remove() {
        let map: AreaMap<Uuid> = AreaMap::new(2);
        let entity = Uuid::new_v4();
        let center = ChunkPos::new(0, 0);

        map.add(entity, center);
        assert_eq!(map.len(), 1);

        map.remove(&entity);
        assert_eq!(map.len(), 0);
        assert!(map.get_entities_tracking_chunk(center).is_empty());
    }

    #[test]
    fn test_move_entity() {
        let map: AreaMap<Uuid> = AreaMap::new(1);
        let entity = Uuid::new_v4();
        let old_center = ChunkPos::new(0, 0);
        let new_center = ChunkPos::new(5, 5);

        map.add(entity, old_center);
        assert!(
            map.get_entities_tracking_chunk(old_center)
                .contains(&entity)
        );

        map.move_entity(&entity, old_center, new_center);

        assert!(
            !map.get_entities_tracking_chunk(old_center)
                .contains(&entity)
        );
        assert!(
            map.get_entities_tracking_chunk(new_center)
                .contains(&entity)
        );
    }

    #[test]
    fn test_multiple_entities() {
        let map: AreaMap<Uuid> = AreaMap::new(2);
        let entity1 = Uuid::new_v4();
        let entity2 = Uuid::new_v4();

        map.add(entity1, ChunkPos::new(0, 0));
        map.add(entity2, ChunkPos::new(1, 1));

        let entities = map.get_entities_tracking_chunk(ChunkPos::new(0, 0));
        assert!(entities.contains(&entity1));
        assert!(entities.contains(&entity2));
        assert_eq!(map.len(), 2);
    }
}
