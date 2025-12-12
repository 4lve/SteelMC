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

/// The struct that holds command handler data and executor.
pub struct CommandHandler {
    /// The name and aliases of this command.
    pub names: &'static [&'static str],
    /// A description of this command.
    pub description: &'static str,
    /// The permission required to execute this command.
    pub permission: &'static str,
    /// The command parser chain for this command.
    executor: Option<Box<dyn CommandParserExecutor<()> + Send + Sync>>,
}

impl CommandHandler {
    /// Creates a new command handler.
    #[must_use]
    pub fn new(
        names: &'static [&'static str],
        description: &'static str,
        permission: &'static str,
    ) -> Self {
        CommandHandler {
            names,
            description,
            permission,
            executor: None,
        }
    }

    /// Chains a command executor to this command handler.
    #[must_use]
    pub fn then(
        mut self,
        executor: impl CommandParserExecutor<()> + Send + Sync + 'static,
    ) -> Self {
        self.executor = Some(Box::new(executor));
        self
    }

    /// Chains a command executor to this command handler.
    #[must_use]
    pub fn executes(
        mut self,
        executor: impl CommandParserExecutor<()> + Send + Sync + 'static,
    ) -> Self {
        self.executor = Some(Box::new(executor));
        self
    }

    /// Handles the execution of a command sent by a player.
    pub fn handle(
        &self,
        command_args: &[&str],
        server: Arc<Server>,
        context: &mut CommandContext,
    ) -> Result<(), CommandError> {
        let Some(executor) = &self.executor else {
            unimplemented!(
                "Command {} has no executor defined. Please call `then()` or `executes()` on the CommandHandler.",
                self.names[0]
            );
        };

        let Some(result) = executor.execute(command_args, (), &server, context) else {
            return Err(CommandError::CommandFailed(Box::new(
                "Invalid Syntax.".into(),
            )));
        };

        result
    }
}

impl<S, A, E> CommandParserExecutor<S> for CommandParserArgumentExecutor<S, A, E>
where
    E: CommandParserExecutor<(A, S)>,
{
    fn execute(
        &self,
        args: &[&str],
        parsed: S,
        server: &Arc<Server>,
        context: &mut CommandContext,
    ) -> Option<Result<(), CommandError>> {
        let (args, arg) = self.argument.parse(args, context)?;
        self.executor.execute(args, (arg, parsed), server, context)
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
        E: CommandParserExecutor<(A, S)>,
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
    ) -> CommandParserArgumentExecutor<S, A, CommandParserLeafExecutor<(A, S), E>>
    where
        E: CommandExecutor<(A, S)>,
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
    ) -> CommandParserArgumentExecutor<S, A, CommandParserSplitExecutor<(A, S), E1, E2>>
    where
        E2: CommandParserExecutor<(A, S)>,
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
    ) -> CommandParserArgumentExecutor<S, A, SplitLeafExecutor<(A, S), E1, E2>>
    where
        E2: CommandExecutor<(A, S)>,
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
