use std::sync::Arc;

use scc::{HashIndex, HashMap};
use steel_utils::{ChunkPos, locks::SteelRwLock};

use crate::{ChunkData, player::Player};

pub struct World {
    pub loaded_chunks: HashIndex<ChunkPos, SteelRwLock<ChunkData>>,
    pub players: HashMap<uuid::Uuid, Arc<Player>>,
}

impl World {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            loaded_chunks: HashIndex::new(),
            players: HashMap::new(),
        }
    }

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
