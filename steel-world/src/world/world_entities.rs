use std::sync::Arc;

use crate::{player::Player, world::World};

impl World {
    pub fn remove_player(&self, player: Arc<Player>) {
        let uuid = player.gameprofile.id;

        if self.players.remove_sync(&uuid).is_some() {
            log::info!("Player {} removed", uuid);
        }
    }

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
