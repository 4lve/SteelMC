//! Handler for the "gamemode" command.
use crate::command::arguments::gamemode::GameModeArgument;
use crate::command::commands::{
    CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn, argument,
};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use steel_utils::translations;
use steel_utils::types::GameType;

/// Handler for the "gamemode" command.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["gamemode"],
        "Sets the game mode.",
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
    ) -> Result<(), CommandError> {
        let ((), gamemode) = args;

        // Get the player executing the command
        let player = context
            .sender
            .get_player()
            .ok_or(CommandError::InvalidRequirement)?;

        // Set the player's game mode
        if !player.set_game_mode(gamemode) {
            // Player was already in the requested game mode
            return Ok(());
        }

        // Send success message
        let mode_translation = match gamemode {
            GameType::Survival => translations::GAME_MODE_SURVIVAL.msg(),
            GameType::Creative => translations::GAME_MODE_CREATIVE.msg(),
            GameType::Adventure => translations::GAME_MODE_ADVENTURE.msg(),
            GameType::Spectator => translations::GAME_MODE_SPECTATOR.msg(),
        };

        context.sender.send_message(
            &translations::COMMANDS_GAMEMODE_SUCCESS_SELF
                .message([mode_translation])
                .into(),
        );

        Ok(())
    }
}
