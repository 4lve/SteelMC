//! This module contains the command building structs.
pub mod weather;

use std::marker::PhantomData;
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
        server: &Arc<Server>,
        context: &mut CommandContext,
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
    fn handle(
        &self,
        command_args: &[&str],
        server: Arc<Server>,
        context: &mut CommandContext,
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

    /// Handles the execution of a command sent by a player.
    fn handle(
        &self,
        command_args: &[&str],
        server: Arc<Server>,
        context: &mut CommandContext,
    ) -> Result<(), CommandError> {
        match self.executor.execute(command_args, (), &server, context) {
            Some(result) => result,
            None => Err(CommandError::CommandFailed(Box::new(
                "Invalid Syntax.".into(),
            ))),
        }
    }

    fn usage(&self, buffer: &mut Vec<CommandNode>, root_children: &mut Vec<i32>) {
        let node_index = buffer.len() as i32;
        let node = CommandNode::new_literal(self.executor.usage(buffer), self.names()[0], None);
        root_children.push(node_index);
        buffer.push(node);

        for name in self.names().iter().skip(1) {
            root_children.push(buffer.len() as i32);
            buffer.push(CommandNode::new_literal(
                CommandNodeInfo::new(Vec::new(), false),
                name,
                Some(node_index),
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
        server: &Arc<Server>,
        context: &mut CommandContext,
    ) -> Option<Result<(), CommandError>>;

    /// Generates usage information for the command.
    fn usage(&self, buffer: &mut Vec<CommandNode>) -> CommandNodeInfo;
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
        server: &Arc<Server>,
        context: &mut CommandContext,
    ) -> Option<Result<(), CommandError>> {
        args.is_empty()
            .then(|| self.executor.execute(parsed, server, context))
    }

    fn usage(&self, _buffer: &mut Vec<CommandNode>) -> CommandNodeInfo {
        CommandNodeInfo::new(Vec::new(), true)
    }
}

/// Tree node that passes execution to the second executor if the first one fails.
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
        server: &Arc<Server>,
        context: &mut CommandContext,
    ) -> Option<Result<(), CommandError>> {
        let result = self
            .first_executor
            .execute(args, parsed.clone(), server, context);
        if result.is_some() {
            return result;
        }

        self.second_executor.execute(args, parsed, server, context)
    }

    fn usage(&self, buffer: &mut Vec<CommandNode>) -> CommandNodeInfo {
        self.first_executor
            .usage(buffer)
            .chain(self.second_executor.usage(buffer))
    }
}

/// A builder struct for creating command argument executors.
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

/// Tree node that parses a single argument and provides it to the next executor.
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
        server: &Arc<Server>,
        context: &mut CommandContext,
    ) -> Option<Result<(), CommandError>> {
        if *args.first()? == self.expected {
            self.executor.execute(&args[1..], parsed, server, context)
        } else {
            None
        }
    }

    fn usage(&self, buffer: &mut Vec<CommandNode>) -> CommandNodeInfo {
        let result = vec![buffer.len() as i32];
        let node = CommandNode::new_literal(self.executor.usage(buffer), self.expected, None);
        buffer.push(node);

        CommandNodeInfo::new(result, false)
    }
}

/// A builder struct for creating command argument executors.
pub struct CommandParserArgumentBuilder<S, A> {
    argument: Box<dyn CommandArgument<Output = A>>,
    _source: PhantomData<S>,
}

/// Creates a new command argument builder.
pub fn argument<S, A>(
    argument: impl CommandArgument<Output = A> + 'static,
) -> CommandParserArgumentBuilder<S, A> {
    CommandParserArgumentBuilder {
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
        server: &Arc<Server>,
        context: &mut CommandContext,
    ) -> Option<Result<(), CommandError>> {
        let (args, arg) = self.argument.parse(args, context)?;
        self.executor.execute(args, (parsed, arg), server, context)
    }

    fn usage(&self, buffer: &mut Vec<CommandNode>) -> CommandNodeInfo {
        let result = vec![buffer.len() as i32];
        let node =
            CommandNode::new_argument(self.executor.usage(buffer), self.argument.usage(), None);
        buffer.push(node);

        CommandNodeInfo::new(result, false)
    }
}

/// Tree node that parses a single argument and provides it to the next executor.
pub struct CommandParserArgumentExecutor<S, A, E> {
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
