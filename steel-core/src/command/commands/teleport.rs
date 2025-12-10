//! Teleport command implementation.
use std::{collections::HashMap, sync::Arc};

use crate::command::arguments::{Arg, CommandParserArgument};
use crate::command::commands::{CommandHandler, CommandParserNode};
use crate::command::context::CommandContext;
use crate::command::error::{CommandError, CommandError::InvalidConsumption};
use crate::server::Server;

/// The names for the "teleport" command.
pub const NAMES: [&str; 2] = ["teleport", "tp"];

const ARG_TARGETS: &str = "targets";
const ARG_DESTINATION: &str = "destination";
const ARG_LOCATION: &str = "location";
const ARG_ROTATION: &str = "rotation";
const ARG_FACING: &str = "facing";
const ARG_FACING_ANCHOR: &str = "anchor";

/// The handler for the "teleport" command.
pub struct TeleportCommandHandler;

impl CommandHandler for TeleportCommandHandler {
    fn get_permission(&self) -> &str {
        "minecraft:command.teleport"
    }

    fn get_tree(&self) -> Box<[CommandParserNode]> {
        Box::new([
            CommandParserNode::Argument {
                name: ARG_DESTINATION,
                parser: CommandParserArgument::Entity,
                children: Box::new([CommandParserNode::Execute]),
            },
            CommandParserNode::Argument {
                name: ARG_LOCATION,
                parser: CommandParserArgument::Pos3D,
                children: Box::new([CommandParserNode::Execute]),
            },
            CommandParserNode::Argument {
                name: ARG_TARGETS,
                parser: CommandParserArgument::Entities,
                children: Box::new([
                    CommandParserNode::Argument {
                        name: ARG_DESTINATION,
                        parser: CommandParserArgument::Entity,
                        children: Box::new([CommandParserNode::Execute]),
                    },
                    CommandParserNode::Argument {
                        name: ARG_LOCATION,
                        parser: CommandParserArgument::Pos3D,
                        children: Box::new([
                            CommandParserNode::Argument {
                                name: ARG_ROTATION,
                                parser: CommandParserArgument::Rotation,
                                children: Box::new([CommandParserNode::Execute]),
                            },
                            CommandParserNode::Argument {
                                name: "facing_literal",
                                parser: CommandParserArgument::Literal("facing"),
                                children: Box::new([
                                    CommandParserNode::Argument {
                                        name: "entity_literal",
                                        parser: CommandParserArgument::Literal("entity"),
                                        children: Box::new([CommandParserNode::Argument {
                                            name: ARG_FACING,
                                            parser: CommandParserArgument::Entity,
                                            children: Box::new([
                                                CommandParserNode::Argument {
                                                    name: ARG_FACING_ANCHOR,
                                                    parser: CommandParserArgument::EntityAnchor,
                                                    children: Box::new([
                                                        CommandParserNode::Execute,
                                                    ]),
                                                },
                                                CommandParserNode::Execute,
                                            ]),
                                        }]),
                                    },
                                    CommandParserNode::Argument {
                                        name: ARG_FACING,
                                        parser: CommandParserArgument::Pos3D,
                                        children: Box::new([CommandParserNode::Execute]),
                                    },
                                ]),
                            },
                            CommandParserNode::Execute,
                        ]),
                    },
                ]),
            },
        ])
    }

    fn execute(
        &self,
        args: HashMap<&'static str, Arg>,
        context: &mut CommandContext,
        _server: Arc<Server>,
    ) -> Result<(), CommandError> {
        let destination: Option<bool> = match args.get(ARG_DESTINATION) {
            // Some(Arg::Entities(targets)) => Some(targets),
            Some(_) => return Err(InvalidConsumption(Some(ARG_DESTINATION.into()))),
            None => None,
        };
        let location = match args.get(ARG_LOCATION) {
            Some(Arg::Pos3D(location)) => Some(location),
            Some(_) => return Err(InvalidConsumption(Some(ARG_LOCATION.into()))),
            None => None,
        };

        match args.get(ARG_TARGETS) {
            // Some(Arg::Entities(targets)) => {
            //     Ok(())
            // },
            Some(_) => Err(InvalidConsumption(Some(ARG_TARGETS.into()))),
            None => {
                if let Some(player) = context.player.take() {
                    if let Some(_destination) = destination {
                        // Teleport to entity
                    } else if let Some(location) = location {
                        player.teleport(*location, None, None);
                    }
                    Ok(())
                } else {
                    Err(CommandError::CommandFailed(
                        "A player is required to run this command here".into(),
                    ))
                }
            }
        }
    }
}
