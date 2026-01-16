//! Handler for the "tellraw" command.
use text_components::TextComponent;

use crate::command::arguments::text::TextArgument;
use crate::command::commands::{
    CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn, argument,
};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::command::sender::CommandSender;
use crate::server::Server;
use std::sync::Arc;

/// Handler for the "tellraw" command.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["tellraw"],
        "Sends a JSON message to players.",
        "minecraft:command.tellraw",
    )
    .then(argument("message", TextArgument).executes(TellrawCommandExecutor))
}

struct TellrawCommandExecutor;

impl CommandExecutor<((), String)> for TellrawCommandExecutor {
    fn execute(
        &self,
        args: ((), String),
        context: &mut CommandContext,
        _server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        let sender = match &context.sender {
            CommandSender::Player(player) => &player.gameprofile.name,
            CommandSender::Console => "Console",
            CommandSender::Rcon => "Rcon",
        };
        match TextComponent::from_snbt(&args.1) {
            Ok(component) => {
                log::info!("{}'s tellraw: {:p}", sender, component);
                context.sender.send_message(&component);
                Ok(())
            }
            Err(e) => {
                log::warn!("{e}");
                return Err(CommandError::InvalidRequirement);
            }
        }
    }
}
