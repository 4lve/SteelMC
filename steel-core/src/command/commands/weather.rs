//! Handler for the "weather" command.
use std::sync::Arc;

use crate::command::arguments::time::TimeArgument;
use crate::command::commands::{CommandExecutor, CommandHandler, argument, literal};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::server::Server;

/// Handler for the "weather" command.
#[must_use]
pub fn command_handler() -> CommandHandler {
    CommandHandler::new(
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

impl CommandExecutor<((), ((), ((), ())))> for WeatherCommandExecutor {
    fn execute(
        &self,
        _args: ((), ((), ((), ()))),
        server: &Arc<Server>,
        context: &mut CommandContext,
    ) -> Result<(), CommandError> {
        let duration = match self {
            WeatherCommandExecutor::Clear => rand::random_range(12_000..=180_000),
            WeatherCommandExecutor::Rain => rand::random_range(12_000..=24_000),
            WeatherCommandExecutor::Thunder => rand::random_range(3_600..=15_600),
        };

        self.execute((duration, ((), ())), server, context)
    }
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

        self.execute((duration, ((), ())), server, context)
    }
}

impl CommandExecutor<(i32, ((), ()))> for WeatherCommandExecutor {
    fn execute(
        &self,
        args: (i32, ((), ())),
        _server: &Arc<Server>,
        _context: &mut CommandContext,
    ) -> Result<(), CommandError> {
        let (duration, _) = args;
        match self {
            WeatherCommandExecutor::Clear => {
                log::info!("Changing weather to clear for {duration} ticks");
            }
            WeatherCommandExecutor::Rain => {
                log::info!("Changing weather to rain for {duration} ticks");
            }
            WeatherCommandExecutor::Thunder => {
                log::info!("Changing weather to thunder for {duration} ticks");
            }
        }
        Ok(())
    }
}
