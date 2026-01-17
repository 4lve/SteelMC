//! Handler for the "tick" command.
use std::sync::Arc;

use steel_utils::text::TextComponent;
use steel_utils::translations;

use crate::command::arguments::time::TimeArgument;
use crate::command::commands::{
    CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn, argument, literal,
};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::server::Server;

/// Handler for the "tick" command.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["tick"],
        "Controls server tick rate.",
        "minecraft:command.tick",
    )
    // /tick query
    .then(literal("query").executes(TickQueryExecutor))
    // /tick freeze
    .then(literal("freeze").executes(TickFreezeExecutor))
    // /tick unfreeze
    .then(literal("unfreeze").executes(TickUnfreezeExecutor))
    // /tick sprint <time> | /tick sprint stop
    .then(
        literal("sprint")
            .then(literal("stop").executes(TickSprintStopExecutor))
            .then(argument("time", TimeArgument).executes(TickSprintExecutor)),
    )
    // TODO: /tick rate <rate> - needs FloatArgument implementation
    // TODO: /tick step [time] - needs stepGameIfPaused() logic in TickRateManager
    // TODO: /tick step stop - needs stopStepping() logic in TickRateManager
    //
    // TODOs requiring GAME LOOP integration:
    // - Sprint report message: When sprint finishes in finish_tick_sprint(), broadcast
    //   translations::COMMANDS_TICK_SPRINT_REPORT with TPS and ms/tick stats
    //   to the command sender (requires storing sender in tick manager or using events)
}

// /tick query
struct TickQueryExecutor;
impl CommandExecutor<()> for TickQueryExecutor {
    fn execute(
        &self,
        _args: (),
        context: &mut CommandContext,
        server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        let tick_manager = server.tick_rate_manager.read();

        let tick_rate = tick_manager.tick_rate;
        let mspt = tick_manager.get_average_mspt();
        let target_mspt = 1000.0 / tick_rate;

        // Get percentiles
        let p50 = tick_manager.get_p50();
        let p95 = tick_manager.get_p95();
        let p99 = tick_manager.get_p99();
        let sample_count = tick_manager.get_sample_count();

        // Send status message
        if tick_manager.is_sprinting() {
            context
                .sender
                .send_message(translations::COMMANDS_TICK_STATUS_SPRINTING.msg().into());
        } else if tick_manager.is_frozen() {
            context
                .sender
                .send_message(translations::COMMANDS_TICK_STATUS_FROZEN.msg().into());
        } else if mspt > target_mspt {
            context
                .sender
                .send_message(translations::COMMANDS_TICK_STATUS_LAGGING.msg().into());
        } else {
            context
                .sender
                .send_message(translations::COMMANDS_TICK_STATUS_RUNNING.msg().into());
        }

        // Send tick rate info
        context.sender.send_message(
            translations::COMMANDS_TICK_QUERY_RATE_RUNNING
                .message([
                    TextComponent::from(format!("{:.1}", tick_rate)),
                    TextComponent::from(format!("{:.1}", mspt)),
                    TextComponent::from(format!("{:.1}", target_mspt)),
                ])
                .into(),
        );

        // Send percentiles
        context.sender.send_message(
            translations::COMMANDS_TICK_QUERY_PERCENTILES
                .message([
                    TextComponent::from(format!("{:.1}", p50)),
                    TextComponent::from(format!("{:.1}", p95)),
                    TextComponent::from(format!("{:.1}", p99)),
                    TextComponent::from(format!("{}", sample_count)),
                ])
                .into(),
        );

        Ok(())
    }
}

// /tick freeze
struct TickFreezeExecutor;
impl CommandExecutor<()> for TickFreezeExecutor {
    fn execute(
        &self,
        _args: (),
        context: &mut CommandContext,
        server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        let mut tick_manager = server.tick_rate_manager.write();

        // Stop sprinting if active
        if tick_manager.is_sprinting() {
            tick_manager.stop_sprinting();
        }

        tick_manager.set_frozen(true);
        drop(tick_manager);

        context
            .sender
            .send_message(translations::COMMANDS_TICK_STATUS_FROZEN.msg().into());

        Ok(())
    }
}

// /tick unfreeze
struct TickUnfreezeExecutor;
impl CommandExecutor<()> for TickUnfreezeExecutor {
    fn execute(
        &self,
        _args: (),
        context: &mut CommandContext,
        server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        server.tick_rate_manager.write().set_frozen(false);

        context
            .sender
            .send_message(translations::COMMANDS_TICK_STATUS_RUNNING.msg().into());

        Ok(())
    }
}

// /tick sprint <time>
struct TickSprintExecutor;
impl CommandExecutor<((), i32)> for TickSprintExecutor {
    fn execute(
        &self,
        args: ((), i32),
        context: &mut CommandContext,
        server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        let ((), ticks) = args;

        let interrupted = server
            .tick_rate_manager
            .write()
            .request_game_to_sprint(ticks as u64);

        if interrupted {
            context
                .sender
                .send_message(translations::COMMANDS_TICK_SPRINT_STOP_SUCCESS.msg().into());
        }

        context
            .sender
            .send_message(translations::COMMANDS_TICK_STATUS_SPRINTING.msg().into());

        Ok(())
    }
}

// /tick sprint stop
struct TickSprintStopExecutor;
impl CommandExecutor<()> for TickSprintStopExecutor {
    fn execute(
        &self,
        _args: (),
        context: &mut CommandContext,
        server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        let stopped = server.tick_rate_manager.write().stop_sprinting();

        if stopped {
            context
                .sender
                .send_message(translations::COMMANDS_TICK_SPRINT_STOP_SUCCESS.msg().into());
        } else {
            return Err(CommandError::CommandFailed(Box::new(
                translations::COMMANDS_TICK_SPRINT_STOP_FAIL.msg().into(),
            )));
        }

        Ok(())
    }
}
