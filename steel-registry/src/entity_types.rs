use rustc_hash::FxHashMap;

use crate::vanilla_entities::{ALL_ENTITY_TYPES, EntityType};

pub type EntityTypeRef = &'static EntityType;

/// Registry for entity types with lookup by ID and key
pub struct EntityTypeRegistry {
    types_by_id: Vec<EntityTypeRef>,
    types_by_key: FxHashMap<&'static str, EntityTypeRef>,
}

impl Default for EntityTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityTypeRegistry {
    /// Creates a new entity type registry populated with all vanilla entity types
    #[must_use]
    pub fn new() -> Self {
        let mut types_by_id = Vec::with_capacity(ALL_ENTITY_TYPES.len());
        let mut types_by_key = FxHashMap::default();

        for entity_type in ALL_ENTITY_TYPES {
            // Ensure the ID matches the index
            debug_assert_eq!(
                entity_type.id as usize,
                types_by_id.len(),
                "Entity type ID mismatch for {}",
                entity_type.key
            );
            types_by_id.push(entity_type);
            types_by_key.insert(entity_type.key, entity_type);
        }

        Self {
            types_by_id,
            types_by_key,
        }
    }

    /// Gets an entity type by its registry ID
    #[must_use]
    pub fn by_id(&self, id: i32) -> Option<EntityTypeRef> {
        if id >= 0 {
            self.types_by_id.get(id as usize).copied()
        } else {
            None
        }
    }

    /// Gets an entity type by its registry key (e.g., "minecraft:player")
    #[must_use]
    pub fn by_key(&self, key: &str) -> Option<EntityTypeRef> {
        self.types_by_key.get(key).copied()
    }

    /// Gets the registry ID for an entity type
    #[must_use]
    pub fn get_id(&self, entity_type: EntityTypeRef) -> i32 {
        entity_type.id
    }

    /// Iterates over all entity types with their IDs
    pub fn iter(&self) -> impl Iterator<Item = (i32, EntityTypeRef)> + '_ {
        self.types_by_id
            .iter()
            .enumerate()
            .map(|(id, &entity_type)| (id as i32, entity_type))
    }

    /// Returns the number of registered entity types
    #[must_use]
    pub fn len(&self) -> usize {
        self.types_by_id.len()
    }

    /// Returns true if the registry is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.types_by_id.is_empty()
    }
}
