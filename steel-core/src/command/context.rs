//! This module contains the command context.
use std::sync::Arc;

use steel_utils::math::Vector3;

use crate::command::sender::CommandSender;
use crate::player::Player;
use crate::server::Server;
use crate::world::World;

/// The context of a command.
#[derive(Clone)]
pub struct CommandContext {
    /// The sender of the command.
    pub sender: CommandSender,
    /// The player targeted by the command.
    pub player: Option<Arc<Player>>,
    /// The world/dimension of the command.
    pub world: Arc<World>,
    /// The server where the command has been run.
    pub server: Arc<Server>,
    /// The position of the command.
    pub position: Vector3<f64>,
    /// The rotation of the command.
    pub rotation: Option<(f32, f32)>,
    /// The anchor of the command.
    pub anchor: EntityAnchor,
}

/// The position anchor to use for an entity.
#[derive(Clone, Default)]
pub enum EntityAnchor {
    /// The feet of the entity.
    #[default]
    Feet,
    /// The eyes of the entity.
    Eyes,
}

impl CommandContext {
    /// Creates a new command context.
    #[must_use]
    pub fn new(sender: CommandSender, server: Arc<Server>) -> Self {
        let player = sender.get_player().cloned();
        let position = player
            .as_ref()
            // TODO: The default position is the surface of the world center
            // (Where the compass should point to)
            .map_or(Vector3::new(0., 63., 0.), |p| *p.position.lock());
        let world = player
            .as_ref()
            .map_or(server.worlds[0].clone(), |p| Arc::clone(&p.world));

        Self {
            sender,
            player,
            world,
            server,
            position,
            rotation: None,
            anchor: EntityAnchor::default(),
        }
    }
}
