//! Handler for the "gamemode" command.
use std::sync::Arc;

use steel_utils::types::GameType;
use steel_utils::{text::TextComponent, translations};

use crate::command::arguments::gamemode::GameModeArgument;
use crate::command::commands::{
    CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn, argument,
};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::server::Server;

/// Handler for the "gamemode" command.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["gamemode"],
        "Changes the game mode of a player.",
        "minecraft:command.gamemode",
    )
    .then(argument("gamemode", GameModeArgument).executes(GameModeCommandExecutor))
}

struct GameModeCommandExecutor;

impl CommandExecutor<((), GameType)> for GameModeCommandExecutor {
    fn execute(
        &self,
        args: ((), GameType),
        context: &mut CommandContext,
        _server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        let ((), game_type) = args;

        let player = context
            .player
            .clone()
            .ok_or(CommandError::InvalidRequirement)?;

        if player.set_game_mode(game_type) {
            let mode_text = game_mode_translation(game_type);
            context.sender.send_message(
                translations::COMMANDS_GAMEMODE_SUCCESS_SELF
                    .message([mode_text])
                    .into(),
            );
        }

        Ok(())
    }
}

/// Returns the translation text component for a game mode.
fn game_mode_translation(game_type: GameType) -> TextComponent {
    match game_type {
        GameType::Survival => translations::GAME_MODE_SURVIVAL.msg().into(),
        GameType::Creative => translations::GAME_MODE_CREATIVE.msg().into(),
        GameType::Adventure => translations::GAME_MODE_ADVENTURE.msg().into(),
        GameType::Spectator => translations::GAME_MODE_SPECTATOR.msg().into(),
    }
}
