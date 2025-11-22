//! This module contains the implementation of the world's entity-related methods.
use std::sync::Arc;

use steel_protocol::packets::game::{CGameEvent, GameEventType};
use tokio::time::Instant;

use crate::{player::Player, world::World};

impl World {
    /// Removes a player from the world.
    pub async fn remove_player(self: &Arc<Self>, player: Arc<Player>) {
        let uuid = player.gameprofile.id;

        if self.players.remove_async(&uuid).await.is_some() {
            let self_clone = self.clone();
            let start = Instant::now();
            self_clone.chunk_map.remove_player(&player);
            player.cleanup();
            log::info!("Player {uuid} removed in {:?}", start.elapsed());
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
            return;
        }

        player.connection.send_packet(CGameEvent {
            event: GameEventType::LevelChunksLoadStart,
            data: 0.0,
        });

        player.connection.send_packet(CGameEvent {
            event: GameEventType::ChangeGameMode,
            data: 1.0,
        });
    }
}
