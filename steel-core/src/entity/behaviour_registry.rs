//! Entity behaviour registry for looking up behaviours by entity type ID.

use std::sync::OnceLock;

use steel_registry::vanilla_entities::{self, ALL_ENTITY_TYPES};

use super::behaviour::{DEFAULT_BEHAVIOUR, DISPLAY_BEHAVIOUR, EntityBehaviour, SLIME_BEHAVIOUR};

/// Registry for entity behaviours, indexed by entity type ID.
pub struct EntityBehaviourRegistry {
    behaviors: Vec<&'static dyn EntityBehaviour>,
}

impl EntityBehaviourRegistry {
    /// Creates a new registry with all vanilla entity behaviours assigned.
    fn new() -> Self {
        let mut behaviors: Vec<&'static dyn EntityBehaviour> =
            Vec::with_capacity(ALL_ENTITY_TYPES.len());

        // Initialize all with default behaviour
        for _ in ALL_ENTITY_TYPES {
            behaviors.push(&DEFAULT_BEHAVIOUR);
        }

        // Assign specific behaviours
        Self::assign_behaviour(&mut behaviors, &vanilla_entities::SLIME, &SLIME_BEHAVIOUR);
        Self::assign_behaviour(
            &mut behaviors,
            &vanilla_entities::MAGMA_CUBE,
            &SLIME_BEHAVIOUR,
        );
        Self::assign_behaviour(
            &mut behaviors,
            &vanilla_entities::BLOCK_DISPLAY,
            &DISPLAY_BEHAVIOUR,
        );
        Self::assign_behaviour(
            &mut behaviors,
            &vanilla_entities::ITEM_DISPLAY,
            &DISPLAY_BEHAVIOUR,
        );
        Self::assign_behaviour(
            &mut behaviors,
            &vanilla_entities::TEXT_DISPLAY,
            &DISPLAY_BEHAVIOUR,
        );

        Self { behaviors }
    }

    /// Helper to assign a behaviour to an entity type.
    fn assign_behaviour(
        behaviors: &mut [&'static dyn EntityBehaviour],
        entity_type: &steel_registry::vanilla_entities::EntityType,
        behaviour: &'static dyn EntityBehaviour,
    ) {
        behaviors[entity_type.id as usize] = behaviour;
    }

    /// Gets the behaviour for an entity type ID.
    ///
    /// Returns the default behaviour if the ID is invalid.
    #[must_use]
    pub fn get_behavior(&self, entity_type_id: i32) -> &'static dyn EntityBehaviour {
        self.behaviors
            .get(entity_type_id as usize)
            .copied()
            .unwrap_or(&DEFAULT_BEHAVIOUR)
    }
}

/// Global registry instance, lazily initialized.
static REGISTRY: OnceLock<EntityBehaviourRegistry> = OnceLock::new();

/// Gets the global entity behaviour registry.
#[must_use]
pub fn get_behaviour_registry() -> &'static EntityBehaviourRegistry {
    REGISTRY.get_or_init(EntityBehaviourRegistry::new)
}
