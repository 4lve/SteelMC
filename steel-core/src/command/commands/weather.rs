//! Handler for the "weather" command.
use std::sync::Arc;

use steel_utils::text::TextComponent;

use crate::command::arguments::time::TimeArgument;
use crate::command::commands::{
    CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn, argument, literal,
};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::server::Server;

/// Handler for the "weather" command.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["weather"],
        "Changes the weather in the current dimension.",
        "minecraft:command.weather",
    )
    .then(
        literal("rain")
            .then(argument(TimeArgument).executes(WeatherCommandExecutor::Rain))
            .executes(WeatherCommandExecutor::Rain),
    )
    .then(
        literal("thunder")
            .then(argument(TimeArgument).executes(WeatherCommandExecutor::Thunder))
            .executes(WeatherCommandExecutor::Thunder),
    )
    .then(
        literal("clear")
            .then(argument(TimeArgument).executes(WeatherCommandExecutor::Clear))
            .executes(WeatherCommandExecutor::Clear),
    )
}

enum WeatherCommandExecutor {
    Clear,
    Rain,
    Thunder,
}

impl CommandExecutor<((), ())> for WeatherCommandExecutor {
    fn execute(
        &self,
        _args: ((), ()),
        server: &Arc<Server>,
        context: &mut CommandContext,
    ) -> Result<(), CommandError> {
        let duration = match self {
            WeatherCommandExecutor::Clear => rand::random_range(12_000..=180_000),
            WeatherCommandExecutor::Rain => rand::random_range(12_000..=24_000),
            WeatherCommandExecutor::Thunder => rand::random_range(3_600..=15_600),
        };

        self.execute((((), ()), duration), server, context)
    }
}

impl CommandExecutor<(((), ()), i32)> for WeatherCommandExecutor {
    fn execute(
        &self,
        args: (((), ()), i32),
        server: &Arc<Server>,
        context: &mut CommandContext,
    ) -> Result<(), CommandError> {
        let (_, _duration) = args;
        let _world = server
            .worlds
            .first()
            .ok_or(CommandError::InvalidRequirement)?;

        match self {
            WeatherCommandExecutor::Clear => {
                context
                    .sender
                    .send_message(TextComponent::const_translate("commands.weather.set.clear"));
            }
            WeatherCommandExecutor::Rain => {
                context
                    .sender
                    .send_message(TextComponent::const_translate("commands.weather.set.rain"));
            }
            WeatherCommandExecutor::Thunder => {
                context.sender.send_message(TextComponent::const_translate(
                    "commands.weather.set.thunder",
                ));
            }
        }

        Ok(())
    }
}
