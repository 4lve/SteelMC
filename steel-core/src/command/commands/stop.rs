//! Handler for the "stop" command.
use std::sync::Arc;

use crate::command::commands::{CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::server::Server;

/// Handler for the "stop" command.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(&["stop"], "Stops the server.", "minecraft:command.stop")
        .executes(StopCommandExecutor)
}

struct StopCommandExecutor;
impl CommandExecutor<()> for StopCommandExecutor {
    fn execute(
        &self,
        _args: (),
        _context: &mut CommandContext,
        server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        server.cancel_token.cancel();
        Ok(())
    }
}
