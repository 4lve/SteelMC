//! Entity registry for creating entity instances.

use std::ops::Deref;
use std::sync::{Arc, OnceLock};

use steel_registry::REGISTRY;
use steel_registry::entity_types::EntityTypeRef;
use steel_registry::vanilla_entities;
use steel_utils::math::Vector3;

use super::SharedEntity;
use super::entities::BlockDisplayEntity;

/// Factory function type for creating entities.
///
/// Takes the entity ID and spawn position, returns a new entity instance.
/// The entity ID should be obtained from `Server::next_entity_id()`.
pub type EntityFactory = fn(i32, Vector3<f64>) -> SharedEntity;

/// Registry entry for an entity type.
struct EntityEntry {
    /// Factory function to create instances.
    factory: Option<EntityFactory>,
}

/// Registry for entity factories.
///
/// Maps `EntityType` to factory functions that can create entity instances.
/// This is used when loading entities from disk or when entities are spawned.
pub struct EntityRegistry {
    entries: Vec<EntityEntry>,
}

impl EntityRegistry {
    /// Creates a new empty registry with entries for all entity types.
    #[must_use]
    pub fn new() -> Self {
        let count = REGISTRY.entity_types.len();
        let entries = (0..count).map(|_| EntityEntry { factory: None }).collect();

        Self { entries }
    }

    /// Registers a factory function for an entity type.
    pub fn register(&mut self, entity_type: EntityTypeRef, factory: EntityFactory) {
        let id = *REGISTRY.entity_types.get_id(entity_type);
        self.entries[id].factory = Some(factory);
    }

    /// Creates a new entity instance.
    ///
    /// Returns `None` if no factory is registered for the given type.
    #[must_use]
    pub fn create(
        &self,
        entity_type: EntityTypeRef,
        entity_id: i32,
        pos: Vector3<f64>,
    ) -> Option<SharedEntity> {
        let id = *REGISTRY.entity_types.get_id(entity_type);
        self.entries.get(id)?.factory.map(|f| f(entity_id, pos))
    }

    /// Returns whether a factory is registered for the given type.
    #[must_use]
    pub fn has_factory(&self, entity_type: EntityTypeRef) -> bool {
        let id = *REGISTRY.entity_types.get_id(entity_type);
        self.entries.get(id).is_some_and(|e| e.factory.is_some())
    }
}

impl Default for EntityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper for the global entity registry that implements `Deref`.
pub struct EntityRegistryLock(OnceLock<EntityRegistry>);

impl Deref for EntityRegistryLock {
    type Target = EntityRegistry;

    fn deref(&self) -> &Self::Target {
        self.0.get().expect("Entity registry not initialized")
    }
}

impl EntityRegistryLock {
    /// Sets the registry. Returns `Err` if already initialized.
    pub fn set(&self, registry: EntityRegistry) -> Result<(), EntityRegistry> {
        self.0.set(registry)
    }
}

/// Global entity registry.
///
/// Access via deref: `ENTITIES.create(type, entity_id, pos)`
pub static ENTITIES: EntityRegistryLock = EntityRegistryLock(OnceLock::new());

/// Initializes the global entity registry.
///
/// This should be called once after the main registry is frozen.
///
/// # Panics
///
/// Panics if called more than once.
pub fn init_entities() {
    let mut registry = EntityRegistry::new();

    // Register block display entity factory
    registry.register(vanilla_entities::BLOCK_DISPLAY, |id, pos| {
        Arc::new(BlockDisplayEntity::new(id, pos))
    });

    assert!(
        ENTITIES.set(registry).is_ok(),
        "Entity registry already initialized"
    );
}
