//! Handler for the "kill" command.

use std::sync::Arc;

use steel_utils::text::TextComponent;

use crate::command::arguments::entity_selector::{EntitySelector, EntitySelectorArgument};
use crate::command::commands::{
    CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn, argument,
};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::server::Server;

//TODO add killing

/// Handler for the "kill" command.
///
/// Syntax:
/// - `/kill` - kill self (the command sender)
/// - `/kill <target>` - kill specified entity/entities
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(&["kill"], "Kills entities.", "minecraft:command.kill")
        // /kill - kills self
        .executes(KillSelfExecutor)
        // /kill <target>
        .then(argument("target", EntitySelectorArgument).executes(KillTargetExecutor))
}

/// Executor for `/kill` - kills the sender
struct KillSelfExecutor;

impl CommandExecutor<()> for KillSelfExecutor {
    fn execute(
        &self,
        _args: (),
        context: &mut CommandContext,
        _server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        let player = context
            .sender
            .get_player()
            .ok_or(CommandError::InvalidRequirement)?;

        let uuid = player.gameprofile.id;
        let removed = player.world.entity_tracker.remove_entity_by_uuid(uuid);

        if removed {
            context.sender.send_message(
                TextComponent::new()
                    .text("Killed ".to_string())
                    .text(player.gameprofile.name.clone()),
            );
            Ok(())
        } else {
            Err(CommandError::CommandFailed(Box::new(
                TextComponent::const_text("Failed to kill entity"),
            )))
        }
    }
}

/// Executor for `/kill <target>` - kills specified target
struct KillTargetExecutor;

impl CommandExecutor<((), EntitySelector)> for KillTargetExecutor {
    fn execute(
        &self,
        args: ((), EntitySelector),
        context: &mut CommandContext,
        _server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        let ((), selector) = args;

        let player = context
            .sender
            .get_player()
            .ok_or(CommandError::InvalidRequirement)?;

        let world = &player.world;

        match selector {
            EntitySelector::CurrentEntity => {
                // @s - kill self
                let uuid = player.gameprofile.id;
                if world.entity_tracker.remove_entity_by_uuid(uuid) {
                    context.sender.send_message(
                        TextComponent::new()
                            .text("Killed ".to_string())
                            .text(player.gameprofile.name.clone()),
                    );
                    Ok(())
                } else {
                    Err(CommandError::CommandFailed(Box::new(
                        TextComponent::const_text("Failed to kill entity"),
                    )))
                }
            }
            EntitySelector::Uuid(uuid) => {
                if world.entity_tracker.remove_entity_by_uuid(uuid) {
                    context
                        .sender
                        .send_message(TextComponent::new().text(format!("Killed entity {uuid}")));
                    Ok(())
                } else {
                    Err(CommandError::CommandFailed(Box::new(
                        TextComponent::new().text(format!("No entity found with UUID {uuid}")),
                    )))
                }
            }
            EntitySelector::NearestPlayer
            | EntitySelector::AllPlayers
            | EntitySelector::RandomPlayer
            | EntitySelector::AllEntities
            | EntitySelector::NearestEntity => {
                // TODO: Implement these selectors
                Err(CommandError::CommandFailed(Box::new(
                    TextComponent::const_text("This selector is not yet implemented"),
                )))
            }
            EntitySelector::PlayerName(name) => {
                // TODO: Look up player by name
                Err(CommandError::CommandFailed(Box::new(
                    TextComponent::new()
                        .text(format!("Player lookup by name not yet implemented: {name}")),
                )))
            }
        }
    }
}
