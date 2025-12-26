//! A game mode argument.
use steel_protocol::packets::game::{ArgumentType, SuggestionType};
use steel_utils::types::GameType;

use crate::command::arguments::CommandArgument;
use crate::command::context::CommandContext;

/// A game mode argument.
pub struct GameModeArgument;

impl CommandArgument for GameModeArgument {
    type Output = GameType;

    fn parse<'a>(
        &self,
        arg: &'a [&'a str],
        _context: &mut CommandContext,
    ) -> Option<(&'a [&'a str], Self::Output)> {
        let s = arg.first()?;

        let game_type = match *s {
            "survival" | "s" | "0" => GameType::Survival,
            "creative" | "c" | "1" => GameType::Creative,
            "adventure" | "a" | "2" => GameType::Adventure,
            "spectator" | "sp" | "3" => GameType::Spectator,
            _ => return None,
        };

        Some((&arg[1..], game_type))
    }

    fn usage(&self) -> (ArgumentType, Option<SuggestionType>) {
        (ArgumentType::Gamemode, None)
    }
}
