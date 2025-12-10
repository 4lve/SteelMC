//! This module contains the command context.
use std::sync::Arc;

use steel_utils::math::Vector3;

use crate::player::Player;

/// The context of a command.
pub struct CommandContext {
    /// The player who sent the command.
    pub player: Option<Arc<Player>>,
    /// The position from which the command was sent.
    pub position: Option<Vector3<f64>>,
}
