//! Handler for the "execute" command.
use std::sync::Arc;

use crate::command::arguments::anchor::AnchorArgument;
use crate::command::arguments::rotation::RotationArgument;
use crate::command::commands::{
    CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn, CommandRedirectTarget, argument,
    literal, redirect,
};
use crate::command::context::{CommandContext, EntityAnchor};
use crate::command::error::CommandError;
use crate::server::Server;

/// Handler for the "execute" command.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["execute"],
        "Executes another command with extra options.",
        "minecraft:command.execute",
    )
    .then(
        literal("anchored").then(
            argument("anchor", AnchorArgument)
                .then(redirect(CommandRedirectTarget::Current, AnchorExecutor)),
        ),
    )
    .then(
        literal("rotated").then(
            argument("rot", RotationArgument)
                .then(redirect(CommandRedirectTarget::Current, RotationExecutor)),
        ),
    )
    .then(literal("run").then(redirect(CommandRedirectTarget::All, RunExecutor)))
}

struct AnchorExecutor;
impl CommandExecutor<((), EntityAnchor)> for AnchorExecutor {
    fn execute(
        &self,
        args: ((), EntityAnchor),
        context: &mut CommandContext,
        _server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        context.anchor = args.1;
        Ok(())
    }
}

struct RotationExecutor;
impl CommandExecutor<((), (f32, f32))> for RotationExecutor {
    fn execute(
        &self,
        args: ((), (f32, f32)),
        context: &mut CommandContext,
        _server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        context.rotation = Some(args.1);
        Ok(())
    }
}

struct RunExecutor;
impl CommandExecutor<()> for RunExecutor {
    fn execute(
        &self,
        _args: (),
        _context: &mut CommandContext,
        _server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        Ok(())
    }
}
