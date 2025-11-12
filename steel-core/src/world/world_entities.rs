//! This module contains the implementation of the world's entity-related methods.
use std::sync::Arc;

use crate::{player::Player, world::World};

impl World {
    /// Removes a player from the world.
    pub fn remove_player(&self, player: Arc<Player>) {
        let uuid = player.gameprofile.id;

        if self.players.remove_sync(&uuid).is_some() {
            log::info!("Player {uuid} removed");
        }
    }

    /// Adds a player to the world.
    pub fn add_player(self: &Arc<Self>, player: Arc<Player>) {
        if self
            .players
            .insert_sync(player.gameprofile.id, player.clone())
            .is_err()
        {
            player.connection.close();
        }
    }
}
