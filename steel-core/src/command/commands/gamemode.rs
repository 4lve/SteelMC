//! GameMode command implementation.
use std::{collections::HashMap, sync::Arc};

use crate::command::arguments::{Arg, CommandParserArgument};
use crate::command::commands::{CommandHandler, CommandParserNode};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::server::Server;

/// The names for the "gamemode" command.
pub const NAMES: [&str; 1] = ["gamemode"];

const ARG_GAMEMODE: &str = "gamemode";
const ARG_TARGETS: &str = "targets";

/// The handler for the "gamemode" command.
pub struct GameModeCommandHandler;

impl CommandHandler for GameModeCommandHandler {
    fn get_permission(&self) -> &str {
        "minecraft:command.gamemode"
    }

    fn get_tree(&self) -> Box<[CommandParserNode]> {
        Box::new([CommandParserNode::Argument {
            name: ARG_GAMEMODE,
            parser: CommandParserArgument::Gamemode,
            children: Box::new([
                CommandParserNode::Argument {
                    name: ARG_TARGETS,
                    parser: CommandParserArgument::Players,
                    children: Box::new([CommandParserNode::Execute]),
                },
                CommandParserNode::Execute,
            ]),
        }])
    }

    fn execute(
        &self,
        args: HashMap<&'static str, Arg>,
        context: &mut CommandContext,
        _server: Arc<Server>,
    ) -> Result<(), CommandError> {
        let Some(Arg::GameMode(_gamemode)) = args.get(ARG_GAMEMODE) else {
            return Err(CommandError::InvalidConsumption(Some(ARG_GAMEMODE.into())));
        };
        let players = match args.get(ARG_TARGETS) {
            Some(Arg::Players(targets)) => targets,
            Some(_) => return Err(CommandError::InvalidConsumption(Some(ARG_TARGETS.into()))),
            None => {
                if let Some(player) = context.player.take() {
                    &vec![player]
                } else {
                    return Err(CommandError::CommandFailed(
                        "A player is required to run this command here".into(),
                    ));
                }
            }
        };

        for _player in players {
            // player.set_gamemode(*gamemode);
        }
        Ok(())
    }
}
