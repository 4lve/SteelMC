//! This module contains the actual command implementations.
pub mod gamemode;
pub mod teleport;

use std::collections::HashMap;
use std::sync::Arc;

use crate::command::arguments::{Arg, CommandParserArgument};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::server::Server;

/// The trait that all command handlers must implement.
pub trait CommandHandler: Send + Sync {
    /// The permission required to execute this command.
    fn get_permission(&self) -> &str;

    /// The command parser tree for this command.
    fn get_tree(&self) -> Box<[CommandParserNode]>;

    /// Executes the command with the given parsed arguments.
    fn execute(
        &self,
        args: HashMap<&'static str, Arg>,
        context: &mut CommandContext,
        server: Arc<Server>,
    ) -> Result<(), CommandError>;

    /// Parses the command arguments according to the command parser tree.
    fn parse<'a>(
        &self,
        command_args: &'a [&'a str],
        context: &mut CommandContext,
    ) -> Option<HashMap<&'static str, Arg<'a>>> {
        let mut args = HashMap::new();
        for node in &self.get_tree() {
            if node.parse_arguments(&command_args, &mut args, context) {
                return Some(args);
            }
        }

        None
    }
}

/// A node in the command parser tree.
pub enum CommandParserNode {
    /// A node that will parse an argument.
    Argument {
        /// The name of this argument.
        /// Used as a key in the `args` HashMap passed to [CommandHandler::execute].
        name: &'static str,
        /// The parser to use for this argument.
        parser: CommandParserArgument,
        /// The child nodes of this node.
        children: Box<[CommandParserNode]>,
    },
    /// A leaf node.
    /// Reaching this node while having parsed every arguments will run the command.
    Execute,
}

impl CommandParserNode {
    fn parse_arguments<'a>(
        &self,
        args: &'a [&'a str],
        parsed: &mut HashMap<&'static str, Arg<'a>>,
        context: &mut CommandContext,
    ) -> bool {
        match self {
            CommandParserNode::Execute => args.is_empty(),
            CommandParserNode::Argument {
                name,
                parser,
                children,
            } => {
                if args.is_empty() {
                    return false;
                }
                let Some((arg, args)) = parser.parse(args, context) else {
                    return false;
                };

                parsed.insert(name, arg);
                for child in children {
                    if child.parse_arguments(args, parsed, context) {
                        return true;
                    }
                }
                parsed.remove(name);
                false
            }
        }
    }
}
