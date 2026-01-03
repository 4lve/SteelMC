//! An entity type argument for the summon command.
use steel_protocol::packets::game::{ArgumentType, SuggestionType};
use steel_registry::vanilla_entities::{self, EntityType};

use crate::command::arguments::CommandArgument;
use crate::command::context::CommandContext;

/// Reference to a static `EntityType`
pub type EntityTypeRef = &'static EntityType;

/// An entity type argument that parses entity types like "minecraft:cow", "minecraft:zombie", etc.
pub struct EntityTypeArgument;

impl CommandArgument for EntityTypeArgument {
    type Output = EntityTypeRef;

    fn parse<'a>(
        &self,
        arg: &'a [&'a str],
        _context: &mut CommandContext,
    ) -> Option<(&'a [&'a str], Self::Output)> {
        let s = arg.first()?;

        // Normalize the key - add "minecraft:" prefix if not present (vanilla behavior)
        let full_key = if s.contains(':') {
            s.to_string()
        } else {
            format!("minecraft:{s}")
        };

        // Search through all entity types
        let entity_type = vanilla_entities::ALL_ENTITY_TYPES
            .iter()
            .find(|e| e.key == full_key)?;

        Some((&arg[1..], entity_type))
    }

    fn usage(&self) -> (ArgumentType, Option<SuggestionType>) {
        (
            ArgumentType::Resource {
                identifier: "minecraft:entity_type",
            },
            Some(SuggestionType::SummonableEntities),
        )
    }
}
