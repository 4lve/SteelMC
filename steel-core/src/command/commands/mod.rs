//! This module contains the command building structs.
pub mod execute;
pub mod gamemode;
pub mod seed;
pub mod stop;
pub mod weather;

use std::marker::PhantomData;
use std::ops::Not;
use std::sync::Arc;

use steel_protocol::packets::game::{CommandNode, CommandNodeInfo};

use crate::command::arguments::CommandArgument;
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::server::Server;

/// A trait that defines the behavior of a type safe command executor.
pub trait CommandExecutor<S> {
    /// Executes the command with the given type safe arguments.
    fn execute(
        &self,
        parsed: S,
        context: &mut CommandContext,
        server: &Arc<Server>,
    ) -> Result<(), CommandError>;
}

/// The builder struct that holds command handler data and executor.
pub struct CommandHandlerBuilder {
    names: &'static [&'static str],
    description: &'static str,
    permission: &'static str,
}

/// The struct that holds command handler data and executor.
pub struct CommandHandler<E> {
    names: &'static [&'static str],
    description: &'static str,
    permission: &'static str,
    executor: E,
}

/// Defines a command handler that can be dynamically dispatched.
pub trait CommandHandlerDyn {
    /// Returns the names of the command.
    fn names(&self) -> &'static [&'static str];

    /// Returns the description of the command.
    fn description(&self) -> &'static str;

    /// Returns the permission of the command.
    fn permission(&self) -> &'static str;

    /// Handles the execution of a command sent by a player.
    fn execute(
        &self,
        command_args: &[&str],
        context: &mut CommandContext,
        server: &Arc<Server>,
    ) -> Result<(), CommandError>;

    /// Generates the usage information for the command.
    fn usage(&self, buffer: &mut Vec<CommandNode>, root_children: &mut Vec<i32>);
}

impl CommandHandlerBuilder {
    /// Creates a new command handler builder.
    #[must_use]
    pub fn new(
        names: &'static [&'static str],
        description: &'static str,
        permission: &'static str,
    ) -> CommandHandlerBuilder {
        CommandHandlerBuilder {
            names,
            description,
            permission,
        }
    }

    /// Chains a command executor to this command handler.
    #[must_use]
    pub fn then<E>(self, executor: E) -> CommandHandler<E>
    where
        E: CommandParserExecutor<()>,
    {
        CommandHandler {
            names: self.names,
            description: self.description,
            permission: self.permission,
            executor,
        }
    }

    /// Executes the command executor if the command was ran without arguments.
    pub fn executes<E>(self, executor: E) -> CommandHandler<CommandParserLeafExecutor<(), E>>
    where
        E: CommandExecutor<()>,
    {
        CommandHandler {
            names: self.names,
            description: self.description,
            permission: self.permission,
            executor: CommandParserLeafExecutor {
                executor,
                _source: PhantomData,
            },
        }
    }
}

impl<E1> CommandHandler<E1> {
    /// Chains a command executor that parses arguments.
    #[must_use]
    pub fn then<E2>(self, executor: E2) -> CommandHandler<CommandParserSplitExecutor<(), E1, E2>>
    where
        E2: CommandParserExecutor<()>,
    {
        CommandHandler {
            names: self.names,
            description: self.description,
            permission: self.permission,
            executor: CommandParserSplitExecutor {
                first_executor: self.executor,
                second_executor: executor,
                _source: PhantomData,
            },
        }
    }

    /// Executes the command executor if the command was ran without arguments.
    pub fn executes<E2>(self, executor: E2) -> CommandHandler<CommandParserLeafExecutor<(), E2>>
    where
        E2: CommandExecutor<()>,
    {
        CommandHandler {
            names: self.names,
            description: self.description,
            permission: self.permission,
            executor: CommandParserLeafExecutor {
                executor,
                _source: PhantomData,
            },
        }
    }
}

impl<E> CommandHandlerDyn for CommandHandler<E>
where
    E: CommandParserExecutor<()>,
{
    /// Returns the names of the command.
    fn names(&self) -> &'static [&'static str] {
        self.names
    }

    /// Returns the description of the command.
    fn description(&self) -> &'static str {
        self.description
    }

    /// Returns the permission of the command.
    fn permission(&self) -> &'static str {
        self.permission
    }

    /// Executes the command with the given unparsed arguments.
    fn execute(
        &self,
        command_args: &[&str],
        context: &mut CommandContext,
        server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        match self
            .executor
            .execute(command_args, (), context, server, self)
        {
            Some(result) => result,
            None => Err(CommandError::CommandFailed(Box::new(
                "Invalid Syntax.".into(),
            ))),
        }
    }

    fn usage(&self, buffer: &mut Vec<CommandNode>, root_children: &mut Vec<i32>) {
        let node_index = buffer.len();
        let node = CommandNode::new_root(); // Reserve spot in buffer before calling children
        buffer.push(node);
        root_children.push(node_index as i32);

        buffer[node_index] = CommandNode::new_literal(
            self.executor.usage(buffer, node_index as i32),
            self.names()[0],
        );

        for name in self.names().iter().skip(1) {
            root_children.push(buffer.len() as i32);
            buffer.push(CommandNode::new_literal(
                CommandNodeInfo::new_redirect(node_index as i32),
                name,
            ));
        }
    }
}

/// A trait that defines the behavior of a type safe command executor.
pub trait CommandParserExecutor<S> {
    /// Executes the command with the given unparsed and parsed arguments.
    fn execute(
        &self,
        args: &[&str],
        parsed: S,
        context: &mut CommandContext,
        server: &Arc<Server>,
        handler: &dyn CommandHandlerDyn,
    ) -> Option<Result<(), CommandError>>;

    /// Generates usage information for the command.
    fn usage(&self, buffer: &mut Vec<CommandNode>, node_index: i32) -> CommandNodeInfo;
}

/// Tree node that executes a command with the given parsed arguments.
pub struct CommandParserLeafExecutor<S, E> {
    executor: E,
    _source: PhantomData<S>,
}

impl<S, E> CommandParserExecutor<S> for CommandParserLeafExecutor<S, E>
where
    E: CommandExecutor<S>,
{
    fn execute(
        &self,
        args: &[&str],
        parsed: S,
        context: &mut CommandContext,
        server: &Arc<Server>,
        _: &dyn CommandHandlerDyn,
    ) -> Option<Result<(), CommandError>> {
        args.is_empty()
            .then(|| self.executor.execute(parsed, context, server))
    }

    fn usage(&self, _buffer: &mut Vec<CommandNode>, _: i32) -> CommandNodeInfo {
        CommandNodeInfo::new_executable()
    }
}

/// Tree node that passes execution to the second executor if the first one fails.
/// This allows for branching command syntax where multiple alternatives can be tried.
pub struct CommandParserSplitExecutor<S, E1, E2> {
    first_executor: E1,
    second_executor: E2,
    _source: PhantomData<S>,
}

impl<S, E1, E2> CommandParserExecutor<S> for CommandParserSplitExecutor<S, E1, E2>
where
    S: Clone,
    E1: CommandParserExecutor<S>,
    E2: CommandParserExecutor<S>,
{
    fn execute(
        &self,
        args: &[&str],
        parsed: S,
        context: &mut CommandContext,
        server: &Arc<Server>,
        handler: &dyn CommandHandlerDyn,
    ) -> Option<Result<(), CommandError>> {
        let result = self
            .first_executor
            .execute(args, parsed.clone(), context, server, handler);
        if result.is_some() {
            return result;
        }

        self.second_executor
            .execute(args, parsed, context, server, handler)
    }

    fn usage(&self, buffer: &mut Vec<CommandNode>, node_index: i32) -> CommandNodeInfo {
        self.first_executor
            .usage(buffer, node_index)
            .chain(self.second_executor.usage(buffer, node_index))
    }
}

/// Tree node that redirects to another node after executing.
/// This allows commands to chain into other commands or recursively into themselves.
pub struct CommandParserRedirectExecutor<S, E> {
    to: CommandRedirectTarget,
    executor: E,
    _source: PhantomData<S>,
}

/// Creates a new command redirect builder.
pub fn redirect<S, E>(
    to: CommandRedirectTarget,
    executor: E,
) -> CommandParserRedirectExecutor<S, E> {
    CommandParserRedirectExecutor {
        to,
        executor,
        _source: PhantomData,
    }
}

/// Target for redirecting command execution after a subcommand executes.
pub enum CommandRedirectTarget {
    /// Redirects to the current `CommandHandler`, allowing any branch of the current command to be executed.
    /// Used for commands that chain into themselves (e.g., `/execute anchored feet execute rotated ~ ~ run ...`).
    Current,
    /// Redirects to the `CommandDispatcher`, allowing any registered command to be executed.
    /// Used for commands that can execute arbitrary other commands (e.g., `/execute run <any command>`).
    All,
}

impl<S, E> CommandParserExecutor<S> for CommandParserRedirectExecutor<S, E>
where
    E: CommandExecutor<S>,
{
    fn execute(
        &self,
        args: &[&str],
        parsed: S,
        context: &mut CommandContext,
        server: &Arc<Server>,
        handler: &dyn CommandHandlerDyn,
    ) -> Option<Result<(), CommandError>> {
        if let Err(err) = self.executor.execute(parsed, context, server) {
            return Some(Err(err));
        }

        args.is_empty().not().then(|| match self.to {
            CommandRedirectTarget::Current => handler.execute(args, context, server),
            CommandRedirectTarget::All => {
                server
                    .command_dispatcher
                    .read()
                    .execute(args[0], &args[1..], context, server)
            }
        })
    }

    fn usage(&self, _buffer: &mut Vec<CommandNode>, node_index: i32) -> CommandNodeInfo {
        CommandNodeInfo::new_redirect(match self.to {
            CommandRedirectTarget::Current => node_index,
            CommandRedirectTarget::All => 0,
        })
    }
}

/// A builder struct for creating command literal argument executors.
/// Literals match exact string values (e.g., "clear", "rain", "thunder" in `/weather <clear|rain|thunder>`).
pub struct CommandParserLiteralBuilder<S> {
    expected: &'static str,
    _source: PhantomData<S>,
}

/// Creates a new literal command argument builder.
#[must_use]
pub fn literal<S>(expected: &'static str) -> CommandParserLiteralBuilder<S> {
    CommandParserLiteralBuilder {
        expected,
        _source: PhantomData,
    }
}

impl<S> CommandParserLiteralBuilder<S> {
    /// Executes the command argument executor after the argument is parsed.
    pub fn then<E>(self, executor: E) -> CommandParserLiteralExecutor<S, E>
    where
        E: CommandParserExecutor<S>,
    {
        CommandParserLiteralExecutor {
            expected: self.expected,
            executor,
            _source: PhantomData,
        }
    }

    /// Executes the command executor after the argument is parsed.
    pub fn executes<E>(
        self,
        executor: E,
    ) -> CommandParserLiteralExecutor<S, CommandParserLeafExecutor<S, E>>
    where
        E: CommandExecutor<S>,
    {
        CommandParserLiteralExecutor {
            expected: self.expected,
            executor: CommandParserLeafExecutor {
                executor,
                _source: PhantomData,
            },
            _source: PhantomData,
        }
    }
}

/// Tree node that parses a single literal string and provides execution to the next executor.
/// The literal must match exactly (case-sensitive).
pub struct CommandParserLiteralExecutor<S, E> {
    expected: &'static str,
    executor: E,
    _source: PhantomData<S>,
}

impl<S, E1> CommandParserLiteralExecutor<S, E1> {
    /// Executes the command argument executor after the argument is parsed.
    pub fn then<E2>(
        self,
        executor: E2,
    ) -> CommandParserLiteralExecutor<S, CommandParserSplitExecutor<S, E1, E2>>
    where
        E2: CommandParserExecutor<S>,
    {
        CommandParserLiteralExecutor {
            expected: self.expected,
            executor: CommandParserSplitExecutor {
                first_executor: self.executor,
                second_executor: executor,
                _source: PhantomData,
            },
            _source: PhantomData,
        }
    }

    /// Executes the command executor after the argument is parsed.
    pub fn executes<E2>(
        self,
        executor: E2,
    ) -> CommandParserLiteralExecutor<S, SplitLeafExecutor<S, E1, E2>>
    where
        E2: CommandExecutor<S>,
    {
        CommandParserLiteralExecutor {
            expected: self.expected,
            executor: CommandParserSplitExecutor {
                first_executor: self.executor,
                second_executor: CommandParserLeafExecutor {
                    executor,
                    _source: PhantomData,
                },
                _source: PhantomData,
            },
            _source: PhantomData,
        }
    }
}

impl<S, E> CommandParserExecutor<S> for CommandParserLiteralExecutor<S, E>
where
    E: CommandParserExecutor<S>,
{
    fn execute(
        &self,
        args: &[&str],
        parsed: S,
        context: &mut CommandContext,
        server: &Arc<Server>,
        handler: &dyn CommandHandlerDyn,
    ) -> Option<Result<(), CommandError>> {
        if *args.first()? == self.expected {
            self.executor
                .execute(&args[1..], parsed, context, server, handler)
        } else {
            None
        }
    }

    fn usage(&self, buffer: &mut Vec<CommandNode>, node_index: i32) -> CommandNodeInfo {
        let node = CommandNode::new_literal(self.executor.usage(buffer, node_index), self.expected);
        let result = vec![buffer.len() as i32];
        buffer.push(node);

        CommandNodeInfo::new(result)
    }
}

/// A builder struct for creating typed command argument executors.
/// Arguments are parsed values (e.g., integers, coordinates, entities) defined by `CommandArgument` implementations.
pub struct CommandParserArgumentBuilder<S, A> {
    name: &'static str,
    argument: Box<dyn CommandArgument<Output = A>>,
    _source: PhantomData<S>,
}

/// Creates a new command argument builder.
pub fn argument<S, A>(
    name: &'static str,
    argument: impl CommandArgument<Output = A> + 'static,
) -> CommandParserArgumentBuilder<S, A> {
    CommandParserArgumentBuilder {
        name,
        argument: Box::new(argument),
        _source: PhantomData,
    }
}

impl<S, A> CommandParserArgumentBuilder<S, A> {
    /// Executes the command argument executor after the argument is parsed.
    pub fn then<E>(self, executor: E) -> CommandParserArgumentExecutor<S, A, E>
    where
        E: CommandParserExecutor<(S, A)>,
    {
        CommandParserArgumentExecutor {
            name: self.name,
            argument: self.argument,
            executor,
            _source: PhantomData,
        }
    }

    /// Executes the command executor after the argument is parsed.
    pub fn executes<E>(
        self,
        executor: E,
    ) -> CommandParserArgumentExecutor<S, A, CommandParserLeafExecutor<(S, A), E>>
    where
        E: CommandExecutor<(S, A)>,
    {
        CommandParserArgumentExecutor {
            name: self.name,
            argument: self.argument,
            executor: CommandParserLeafExecutor {
                executor,
                _source: PhantomData,
            },
            _source: PhantomData,
        }
    }
}

impl<S, A, E> CommandParserExecutor<S> for CommandParserArgumentExecutor<S, A, E>
where
    E: CommandParserExecutor<(S, A)>,
{
    fn execute(
        &self,
        args: &[&str],
        parsed: S,
        context: &mut CommandContext,
        server: &Arc<Server>,
        handler: &dyn CommandHandlerDyn,
    ) -> Option<Result<(), CommandError>> {
        let (args, arg) = self.argument.parse(args, context)?;
        self.executor
            .execute(args, (parsed, arg), context, server, handler)
    }

    fn usage(&self, buffer: &mut Vec<CommandNode>, node_index: i32) -> CommandNodeInfo {
        let node = CommandNode::new_argument(
            self.executor.usage(buffer, node_index),
            self.name,
            self.argument.usage(),
        );
        let result = vec![buffer.len() as i32];
        buffer.push(node);

        CommandNodeInfo::new(result)
    }
}

/// Tree node that parses a typed argument and provides the parsed value to the next executor.
/// The argument type `A` is determined by the `CommandArgument` implementation.
pub struct CommandParserArgumentExecutor<S, A, E> {
    name: &'static str,
    argument: Box<dyn CommandArgument<Output = A>>,
    executor: E,
    _source: PhantomData<S>,
}

impl<S, A, E1> CommandParserArgumentExecutor<S, A, E1> {
    /// Executes the command argument executor after the argument is parsed.
    pub fn then<E2>(
        self,
        executor: E2,
    ) -> CommandParserArgumentExecutor<S, A, CommandParserSplitExecutor<(S, A), E1, E2>>
    where
        E2: CommandParserExecutor<(S, A)>,
    {
        CommandParserArgumentExecutor {
            name: self.name,
            argument: self.argument,
            executor: CommandParserSplitExecutor {
                first_executor: self.executor,
                second_executor: executor,
                _source: PhantomData,
            },
            _source: PhantomData,
        }
    }

    /// Executes the command executor after the argument is parsed.
    pub fn executes<E2>(
        self,
        executor: E2,
    ) -> CommandParserArgumentExecutor<S, A, SplitLeafExecutor<(S, A), E1, E2>>
    where
        E2: CommandExecutor<(S, A)>,
    {
        CommandParserArgumentExecutor {
            name: self.name,
            argument: self.argument,
            executor: CommandParserSplitExecutor {
                first_executor: self.executor,
                second_executor: CommandParserLeafExecutor {
                    executor,
                    _source: PhantomData,
                },
                _source: PhantomData,
            },
            _source: PhantomData,
        }
    }
}

type SplitLeafExecutor<S, E1, E2> =
    CommandParserSplitExecutor<S, E1, CommandParserLeafExecutor<S, E2>>;
