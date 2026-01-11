//! This module contains the implementation of the world's entity-related methods.
use std::sync::Arc;

use steel_protocol::packets::game::{
    CAddEntity, CGameEvent, CPlayerInfoUpdate, CRemoveEntities, CRemovePlayerInfo, CSetEntityData,
    GameEventType,
};

use crate::entity::entity_data_to_packet_entries;
use steel_utils::ChunkPos;
use tokio::time::Instant;

use crate::{
    chunk::player_chunk_view::PlayerChunkView, config::STEEL_CONFIG, player::Player, world::World,
};

impl World {
    /// Removes a player from the world.
    pub async fn remove_player(self: &Arc<Self>, player: Arc<Player>) {
        let uuid = player.gameprofile.id;
        let entity_id = player.entity_id;

        if self.players.remove_async(&uuid).await.is_some() {
            let start = Instant::now();

            self.player_area_map.on_player_leave(&player);
            let remove_entity = CRemoveEntities::single(entity_id);
            let remove_info = CRemovePlayerInfo::single(uuid);
            self.players.iter_sync(|_, p| {
                p.connection.send_packet(remove_entity.clone());
                p.connection.send_packet(remove_info.clone());
                true
            });

            self.chunk_map.remove_player(&player);
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

        let pos = *player.position.lock();

        // Register in area map before reading existing positions to receive their movement broadcasts.
        // Don't set last_tracking_view so chunk loading still runs in update_player_status.
        let chunk_pos = ChunkPos::new((pos.x as i32) >> 4, (pos.z as i32) >> 4);
        let view = PlayerChunkView::new(chunk_pos, STEEL_CONFIG.view_distance);
        self.player_area_map.on_player_join(&player, &view);
        let (yaw, pitch) = player.rotation.load();

        // Send existing players to the new player (tab list + entity spawn)
        self.players.iter_sync(|_, existing_player| {
            if existing_player.gameprofile.id != player.gameprofile.id {
                // Add to tab list
                let add_existing = CPlayerInfoUpdate::add_player(
                    existing_player.gameprofile.id,
                    existing_player.gameprofile.name.clone(),
                    existing_player.gameprofile.properties.clone(),
                );
                player.connection.send_packet(add_existing);

                // Send chat session if available
                if let Some(session) = existing_player.chat_session()
                    && let Ok(protocol_data) = session.as_data().to_protocol_data()
                {
                    let session_packet = CPlayerInfoUpdate::update_chat_session(
                        existing_player.gameprofile.id,
                        protocol_data,
                    );
                    player.connection.send_packet(session_packet);
                }

                // Spawn existing player entity for new player
                let existing_pos = *existing_player.position.lock();
                let (existing_yaw, existing_pitch) = existing_player.rotation.load();
                player.connection.send_packet(CAddEntity::player(
                    existing_player.entity_id,
                    existing_player.gameprofile.id,
                    existing_pos.x,
                    existing_pos.y,
                    existing_pos.z,
                    existing_yaw,
                    existing_pitch,
                ));

                // Send existing player's entity data (pose, flags, etc.)
                let entity_data = existing_player.pack_entity_data();
                if !entity_data.is_empty() {
                    player.connection.send_packet(CSetEntityData {
                        entity_id: existing_player.entity_id,
                        metadata: entity_data_to_packet_entries(entity_data),
                    });
                }
            }
            true
        });

        // Broadcast new player to all existing players (tab list + entity spawn)
        let player_info_packet = CPlayerInfoUpdate::add_player(
            player.gameprofile.id,
            player.gameprofile.name.clone(),
            player.gameprofile.properties.clone(),
        );
        let spawn_packet = CAddEntity::player(
            player.entity_id,
            player.gameprofile.id,
            pos.x,
            pos.y,
            pos.z,
            yaw,
            pitch,
        );

        // Prepare new player's entity data
        let new_player_entity_data = player.pack_entity_data();
        let entity_data_packet = if new_player_entity_data.is_empty() {
            None
        } else {
            Some(CSetEntityData {
                entity_id: player.entity_id,
                metadata: entity_data_to_packet_entries(new_player_entity_data),
            })
        };

        self.players.iter_sync(|_, p| {
            p.connection.send_packet(player_info_packet.clone());
            // Don't send spawn packet to self
            if p.gameprofile.id != player.gameprofile.id {
                p.connection.send_packet(spawn_packet.clone());
                if let Some(ref data_packet) = entity_data_packet {
                    p.connection.send_packet(data_packet.clone());
                }
            }
            true
        });

        player.connection.send_packet(CGameEvent {
            event: GameEventType::LevelChunksLoadStart,
            data: 0.0,
        });

        player.connection.send_packet(CGameEvent {
            event: GameEventType::ChangeGameMode,
            data: player.game_mode.load().into(),
        });
    }
}
