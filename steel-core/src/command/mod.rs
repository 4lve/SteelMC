//! This module contains everything needed for commands (e.g., parsing, execution, and sender handling).
pub mod arguments;
pub mod commands;
pub mod context;
pub mod error;
pub mod sender;

use std::sync::Arc;

use steel_utils::text::{TextComponent, color::NamedColor};

use crate::command::commands::CommandHandlerDyn;
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::command::sender::CommandSender;
use crate::server::Server;

/// A struct that parses and dispatches commands to their appropriate handlers.
#[derive(Default)]
pub struct CommandDispatcher {
    /// A map of command names to their handlers.
    handlers: scc::HashMap<&'static str, Arc<dyn CommandHandlerDyn + Send + Sync>>,
}

impl CommandDispatcher {
    /// Creates a new command dispatcher with vanilla handlers.
    #[must_use]
    pub fn new() -> Self {
        let dispatcher = CommandDispatcher::new_empty();
        // dispatcher.register(commands::gamemode::command_handler());
        // dispatcher.register(commands::teleport::command_handler());
        dispatcher.register(commands::weather::command_handler());
        dispatcher
    }

    /// Creates a new command dispatcher with no handlers.
    #[must_use]
    pub fn new_empty() -> Self {
        CommandDispatcher {
            handlers: scc::HashMap::new(),
        }
    }

    /// Executes a command.
    pub fn handle_command(&self, sender: CommandSender, command: String, server: Arc<Server>) {
        let mut context = CommandContext {
            sender: sender.clone(),
            player: sender.get_player().cloned(),
            position: sender.get_player().map(|p| *p.position.lock()),
        };

        if let Err(error) = self.execute(&mut context, &command, server) {
            let text = match error {
                CommandError::InvalidConsumption(s) => {
                    log::error!(
                        "Error while parsing command \"{command}\": {s:?} was consumed, but couldn't be parsed"
                    );
                    TextComponent::const_text("Internal error (See logs for details)")
                }
                CommandError::InvalidRequirement => {
                    log::error!(
                        "Error while parsing command \"{command}\": a requirement that was expected was not met."
                    );
                    TextComponent::const_text("Internal error (See logs for details)")
                }
                CommandError::PermissionDenied => {
                    log::warn!("Permission denied for command \"{command}\"");
                    TextComponent::const_text(
                        "I'm sorry, but you do not have permission to perform this command. Please contact the server administrator if you believe this is an error.",
                    )
                }
                CommandError::CommandFailed(text_component) => *text_component,
            };

            // TODO: Use vanilla error messages
            sender.send_message(text.color(NamedColor::Red));
        }
    }

    /// Executes a command.
    fn execute(
        &self,
        context: &mut CommandContext,
        command: &str,
        server: Arc<Server>,
    ) -> Result<(), CommandError> {
        let (command, command_args) = Self::split_command(command)?;

        let Some(handler) = self.handlers.read_sync(command, |_, v| v.clone()) else {
            return Err(CommandError::CommandFailed(Box::new(
                format!("Command {command} does not exist").into(),
            )));
        };

        // TODO: Implement permission checking logic here
        // if let CommandSender::Player(ref player) = sender
        //     && !server.player_has_permission(player, &handler.permission)
        // {
        //     return Err(PermissionDenied);
        // };

        handler.handle(&command_args, server, context)
    }

    /// Parses a command string into its components.
    fn split_command(command: &str) -> Result<(&str, Box<[&str]>), CommandError> {
        let command = command.trim();
        if command.is_empty() {
            return Err(CommandError::CommandFailed(Box::new(
                TextComponent::const_text("Empty Command"),
            )));
        }

        let Some((command, command_args)) = command.split_once(' ') else {
            return Ok((command, Box::new([])));
        };

        // TODO: Implement proper command parsing (handling quotes, escapes, etc.)

        Ok((command, command_args.split_whitespace().collect()))
    }

    /// Registers a command handler.
    pub fn register(&self, handler: impl CommandHandlerDyn + Send + Sync + 'static) {
        let handler = Arc::new(handler);
        for name in handler.names() {
            if let Err((name, _)) = self.handlers.insert_sync(name, handler.clone()) {
                log::warn!("Command {name} is already registered");
            }
        }
    }

    /// Unregisters a command handler.
    pub fn unregister(&self, names: &[&'static str]) {
        for name in names {
            self.handlers.remove_sync(name);
        }
    }
}
