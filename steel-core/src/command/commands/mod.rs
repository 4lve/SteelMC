//! This module contains the command building structs.
pub mod weather;

use std::marker::PhantomData;
use std::sync::Arc;

use crate::command::arguments::CommandArgument;
use crate::command::arguments::literal::LiteralArgument;
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
}

/// Tree node that parses a single argument and provides it to the next executor.
pub struct CommandParserArgumentExecutor<S, A, E> {
    argument: Box<dyn CommandArgument<Output = A>>,
    executor: E,
    _source: PhantomData<S>,
}

/// Tree node that passes execution to the second executor if the first one fails.
pub struct CommandParserSplitExecutor<S, E1, E2> {
    first_executor: E1,
    second_executor: E2,
    _source: PhantomData<S>,
}

/// Tree node that executes a command with the given parsed arguments.
pub struct CommandParserLeafExecutor<S, E> {
    executor: E,
    _source: PhantomData<S>,
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

/// Creates a new literal command argument builder.
#[must_use]
pub fn literal<S>(expected: &'static str) -> CommandParserArgumentBuilder<S, ()> {
    CommandParserArgumentBuilder {
        argument: Box::new(LiteralArgument { expected }),
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
