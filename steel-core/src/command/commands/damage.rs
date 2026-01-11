//! Handler for the "damage" command.
use std::sync::Arc;

use crate::command::arguments::float::FloatArgument;
use crate::command::commands::{CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn, argument};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::entity::LivingEntity;
use crate::server::Server;
use steel_utils::text::TextComponent;

/// Handler for the "damage" command.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["damage"],
        "Applies damage to the executing player.",
        "minecraft:command.damage",
    )
    .then(argument("amount", FloatArgument { min: 0.0 }).executes(DamageCommandExecutor))
}

struct DamageCommandExecutor;

impl CommandExecutor<((), f32)> for DamageCommandExecutor {
    fn execute(
        &self,
        args: ((), f32),
        context: &mut CommandContext,
        _server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        let ((), amount) = args;

        // Get the player executing the command
        let player = context
            .sender
            .get_player()
            .ok_or(CommandError::InvalidRequirement)?;

        // Get current health and apply damage
        let current_health = player.get_health();
        let new_health = (current_health - amount).max(0.0);

        // Set the new health (this will automatically sync to client via CSetHealth)
        player.set_health(new_health);

        // Send success message
        context.sender.send_message(
            TextComponent::new()
                .text(format!("Applied {} damage to {}", amount, player.gameprofile.name))
                .into(),
        );

        Ok(())
    }
}