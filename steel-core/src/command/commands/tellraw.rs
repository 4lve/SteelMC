//! Handler for the "tellraw" command.
use text_components::TextComponent;

use crate::command::arguments::entity::EntityArgument;
use crate::command::arguments::text_component::TextComponentArgument;
use crate::command::commands::{
    CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn, argument,
};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::command::sender::CommandSender;
use crate::entity::LivingEntity;
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
    .then(
        argument("targets", EntityArgument::entities())
            .then(argument("message", TextComponentArgument).executes(TellrawCommandExecutor)),
    )
}

struct TellrawCommandExecutor;

impl
    CommandExecutor<(
        ((), Vec<Arc<dyn LivingEntity + Send + Sync>>),
        TextComponent,
    )> for TellrawCommandExecutor
{
    fn execute(
        &self,
        args: (
            ((), Vec<Arc<dyn LivingEntity + Send + Sync>>),
            TextComponent,
        ),
        context: &mut CommandContext,
        _server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        let sender = match &context.sender {
            CommandSender::Player(player) => &player.gameprofile.name,
            CommandSender::Console => "Console",
            CommandSender::Rcon => "Rcon",
        };
        log::info!("{}'s tellraw: {:p}", sender, &args.1);
        for entity in args.0.1 {
            if let Some(player) = entity.as_player() {
                player.send_message(&args.1);
            }
        }
        Ok(())
    }
}
