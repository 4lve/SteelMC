use std::sync::Arc;

use crate::{player::Player, world::World};

impl World {
    pub fn player_removed_listener(self: &Arc<Self>, player: Arc<Player>) {
        let uuid = player.game_profile.id;
        let world = self.clone();
        tokio::spawn(async move {
            player.cancel_token.cancelled().await;
            if world.players.remove_sync(&uuid).is_some() {
                log::info!("Player {} removed", uuid);
            }
        });
    }

    pub fn add_player(self: &Arc<Self>, player: Arc<Player>) {
        if self
            .players
            .insert_sync(player.game_profile.id, player.clone())
            .is_err()
        {
            player.cancel_token.cancel();
            return;
        }
        self.player_removed_listener(player);
    }
}
