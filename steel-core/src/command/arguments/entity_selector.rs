//! Entity selector argument parser for commands.
//!
//! Based on Minecraft's `EntitySelectorParser`.

use steel_protocol::packets::game::{ArgumentType, SuggestionType};
use uuid::Uuid;

use super::CommandArgument;
use crate::command::context::CommandContext;

/// The result of parsing an entity selector.
#[derive(Debug, Clone)]
pub enum EntitySelector {
    /// The command sender themselves (`@s`)
    CurrentEntity,
    /// Nearest player (`@p`)
    NearestPlayer,
    /// All players (`@a`)
    AllPlayers,
    /// Random player (`@r`)
    RandomPlayer,
    /// All entities (`@e`)
    AllEntities,
    /// Nearest entity (`@n`)
    NearestEntity,
    /// A specific entity by UUID
    Uuid(Uuid),
    /// A player by name
    PlayerName(String),
}

/// Parses an entity selector argument from the command.
///
/// Supports:
/// - `@s` - the command sender
/// - `@p` - nearest player
/// - `@a` - all players
/// - `@r` - random player
/// - `@e` - all entities
/// - `@n` - nearest entity
/// - `<uuid>` - specific entity by UUID
/// - `<name>` - player by name (1-16 chars)
///
/// TODO: Options like `@e[type=zombie,distance=..10]`
pub struct EntitySelectorArgument;

impl CommandArgument for EntitySelectorArgument {
    type Output = EntitySelector;

    fn parse<'a>(
        &self,
        arg: &'a [&'a str],
        _context: &mut CommandContext,
    ) -> Option<(&'a [&'a str], Self::Output)> {
        let s = arg.first()?;

        // Check for selector syntax (@X)
        if let Some(selector_char) = s.strip_prefix('@') {
            // TODO: Handle options like @e[type=zombie]
            // For now, just parse the base selector
            let base = selector_char.chars().next()?;
            let result = match base {
                's' => EntitySelector::CurrentEntity,
                'p' => EntitySelector::NearestPlayer,
                'a' => EntitySelector::AllPlayers,
                'r' => EntitySelector::RandomPlayer,
                'e' => EntitySelector::AllEntities,
                'n' => EntitySelector::NearestEntity,
                _ => return None, // Unknown selector
            };
            return Some((&arg[1..], result));
        }

        // Try parsing as UUID
        if let Ok(uuid) = Uuid::parse_str(s) {
            return Some((&arg[1..], EntitySelector::Uuid(uuid)));
        }

        // Try as player name (1-16 chars, like vanilla)
        if !s.is_empty() && s.len() <= 16 {
            return Some((&arg[1..], EntitySelector::PlayerName((*s).to_string())));
        }

        None
    }

    fn usage(&self) -> (ArgumentType, Option<SuggestionType>) {
        // Entity flags: bit 0 = single only, bit 1 = players only
        // 0x00 = multiple entities allowed, all entity types
        (ArgumentType::Entity { flags: 0x00 }, None)
    }
}
