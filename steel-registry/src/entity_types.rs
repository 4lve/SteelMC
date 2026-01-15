use rustc_hash::FxHashMap;

use crate::RegistryExt;

#[derive(Debug)]
pub struct EntityType {
    pub key: &'static str,
    pub id: i32,
    pub client_tracking_range: i32,
    pub update_interval: i32,
}

pub type EntityTypeRef = &'static EntityType;

pub struct EntityTypeRegistry {
    types_by_id: Vec<EntityTypeRef>,
    types_by_key: FxHashMap<&'static str, usize>,
    allows_registering: bool,
}

impl Default for EntityTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityTypeRegistry {
    // Creates a new, empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            types_by_id: Vec::new(),
            types_by_key: FxHashMap::default(),
            allows_registering: true,
        }
    }

    /// Registers a new entity type
    pub fn register(&mut self, entity_type: EntityTypeRef) {
        assert!(
            self.allows_registering,
            "Cannot register entity types after the registry has been frozen"
        );
        let idx = self.types_by_id.len();
        self.types_by_key.insert(entity_type.key, idx);
        self.types_by_id.push(entity_type);
    }

    #[must_use]
    pub fn by_id(&self, id: i32) -> Option<EntityTypeRef> {
        if id >= 0 {
            self.types_by_id.get(id as usize).copied()
        } else {
            None
        }
    }

    #[must_use]
    pub fn by_key(&self, key: &str) -> Option<EntityTypeRef> {
        self.types_by_key
            .get(key)
            .and_then(|&idx| self.types_by_id.get(idx).copied())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.types_by_id.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.types_by_id.is_empty()
    }
}

impl RegistryExt for EntityTypeRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}
