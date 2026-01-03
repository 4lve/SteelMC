//! Tracked entity wrapper
//!
//! Wraps an entity and tracks which players can see it, handling
//! spawn/despawn packets and synchronization.

use rustc_hash::FxHashMap;
use std::sync::Arc;
use uuid::Uuid;

use super::{Entity, packet_helpers::entity_data_to_packet_entries};
use crate::player::Player;
use steel_protocol::packets::game::{
    CAddEntity, CMoveEntityPos, CMoveEntityPosRot, CMoveEntityRot, CRemoveEntities, CRotateHead,
    CSetEntityData, CTeleportEntity,
};
use steel_registry::vanilla_entities;
use steel_utils::codec::VarInt;
use steel_utils::locks::{SyncMutex, SyncRwLock};

/// Wrapper around an entity that tracks visibility to players
pub struct TrackedEntity {
    /// The entity being tracked
    pub entity: Arc<dyn Entity>,

    /// Players that can currently see this entity (UUID -> Player)
    seen_by: SyncRwLock<FxHashMap<Uuid, Arc<Player>>>,

    /// Tracking range in blocks
    pub tracking_range_blocks: i32,

    /// Last synced position
    last_position: SyncRwLock<steel_utils::math::Vector3<f64>>,

    /// Last synced rotation
    last_rotation: SyncRwLock<(f32, f32)>,

    /// Update interval in ticks (how often to broadcast changes)
    update_interval: u8,

    /// Tick counter for update intervals
    tick_count: SyncMutex<u64>,
}

impl TrackedEntity {
    /// Creates a new tracked entity
    pub fn new(entity: Arc<dyn Entity>, tracking_range_blocks: i32) -> Self {
        let position = entity.position();
        let rotation = entity.rotation();

        Self {
            entity,
            seen_by: SyncRwLock::new(FxHashMap::default()),
            tracking_range_blocks,
            last_position: SyncRwLock::new(position),
            last_rotation: SyncRwLock::new(rotation),
            update_interval: 1, // Update every tick by default
            tick_count: SyncMutex::new(0),
        }
    }

    /// Adds a player to the seen-by list and sends spawn packets
    pub fn add_player(&self, player: Arc<Player>) {
        let player_uuid = player.gameprofile.id;
        let mut seen_by = self.seen_by.write();

        // Check if already tracking
        if seen_by.contains_key(&player_uuid) {
            return;
        }

        seen_by.insert(player_uuid, player.clone());
        drop(seen_by);

        // Send pairing packets (spawn + initial data)
        self.send_pairing_data(&player);

        // Notify entity
        self.entity.start_seen_by_player(player_uuid);
    }

    /// Removes a player from the seen-by list and sends despawn packet
    pub fn remove_player(&self, player_uuid: Uuid) {
        let mut seen_by = self.seen_by.write();

        // Remove the player and send removal packet
        if let Some(player) = seen_by.remove(&player_uuid) {
            drop(seen_by);
            self.send_removal_packet(&player);
            self.entity.remove_seen_by_player(player_uuid);
        }
    }

    /// Sends spawn and initial data to a player
    fn send_pairing_data(&self, player: &Player) {
        let entity_id = self.entity.entity_id();
        let entity_uuid = self.entity.uuid();

        // Send absolute position
        let position = self.entity.position();
        let (yaw, pitch) = self.entity.rotation();

        // Update last_position to match what we're about to send
        // This ensures future delta calculations are correct
        *self.last_position.write() = position;
        *self.last_rotation.write() = (yaw, pitch);

        // Send CAddEntity to tell the client about this entity
        let add_entity_packet = CAddEntity {
            entity_id,
            uuid: entity_uuid,
            entity_type: VarInt(vanilla_entities::PLAYER.id),
            x: position.x,
            y: position.y,
            z: position.z,
            pitch: (pitch * 256.0 / 360.0) as i8,
            yaw: (yaw * 256.0 / 360.0) as i8,
            head_yaw: (yaw * 256.0 / 360.0) as i8,
            data: VarInt(0),
        };
        player.connection.send_packet(add_entity_packet);

        // Send entity metadata
        let entity_data = self.entity.entity_data();
        let all_data = entity_data.pack_all();
        if !all_data.is_empty() {
            let metadata_packet = CSetEntityData {
                entity_id,
                metadata: entity_data_to_packet_entries(all_data),
            };
            player.connection.send_packet(metadata_packet);
        }
    }

    /// Sends entity removal packet to a player
    fn send_removal_packet(&self, player: &Player) {
        let packet = CRemoveEntities {
            entity_ids: vec![self.entity.entity_id()],
        };
        player.connection.send_packet(packet);
    }

    /// Broadcasts removal to all tracking players
    pub fn broadcast_removal(&self) {
        let seen_by = self.seen_by.read();
        for player in seen_by.values() {
            self.send_removal_packet(player);
        }
    }

    /// Sends position/rotation/data updates to tracking players
    #[allow(clippy::too_many_lines)]
    pub fn send_changes(&self) {
        let mut tick_count = self.tick_count.lock();
        *tick_count += 1;

        // Only update at specified interval
        if !(*tick_count).is_multiple_of(u64::from(self.update_interval)) {
            return;
        }

        let current_pos = self.entity.position();
        let current_rot = self.entity.rotation();
        let mut last_pos = self.last_position.write();
        let mut last_rot = self.last_rotation.write();

        let pos_changed = *last_pos != current_pos;
        let rot_changed = *last_rot != current_rot;

        let seen_by = self.seen_by.read();
        if seen_by.is_empty() {
            // No one watching, just update last_position to keep it current
            if pos_changed || rot_changed {
                *last_pos = current_pos;
                *last_rot = current_rot;
            }
            return;
        }

        // Send position/rotation updates if changed
        if pos_changed || rot_changed {
            let entity_id = self.entity.entity_id();

            // Check if movement is too large for delta encoding (max ~8 blocks)
            // Delta encoding uses i16 with formula: (pos * 32 - last_pos * 32) * 128
            // Max i16 value 32767 / 128 / 32 = ~8 blocks
            let dx = (current_pos.x - last_pos.x).abs();
            let dy = (current_pos.y - last_pos.y).abs();
            let dz = (current_pos.z - last_pos.z).abs();
            let max_delta = dx.max(dy).max(dz);

            if pos_changed && max_delta > 8.0 {
                // Movement too large, use teleport packet instead
                let velocity = self.entity.delta_movement();
                let packet = CTeleportEntity {
                    entity_id,
                    x: current_pos.x,
                    y: current_pos.y,
                    z: current_pos.z,
                    delta_x: velocity.x,
                    delta_y: velocity.y,
                    delta_z: velocity.z,
                    yaw: current_rot.0,
                    pitch: current_rot.1,
                    relatives: 0, // All absolute positioning
                    on_ground: true,
                };

                for player in seen_by.values() {
                    player.connection.send_packet(packet.clone());
                }
            } else if pos_changed && rot_changed {
                // Both position and rotation changed
                let delta_x = ((current_pos.x * 32.0 - last_pos.x * 32.0) * 128.0) as i16;
                let delta_y = ((current_pos.y * 32.0 - last_pos.y * 32.0) * 128.0) as i16;
                let delta_z = ((current_pos.z * 32.0 - last_pos.z * 32.0) * 128.0) as i16;

                let packet = CMoveEntityPosRot {
                    entity_id,
                    delta_x,
                    delta_y,
                    delta_z,
                    yaw: (current_rot.0 * 256.0 / 360.0) as i8,
                    pitch: (current_rot.1 * 256.0 / 360.0) as i8,
                    on_ground: true,
                };

                for player in seen_by.values() {
                    player.connection.send_packet(packet.clone());
                }
            } else if pos_changed {
                // Only position changed
                let delta_x = ((current_pos.x * 32.0 - last_pos.x * 32.0) * 128.0) as i16;
                let delta_y = ((current_pos.y * 32.0 - last_pos.y * 32.0) * 128.0) as i16;
                let delta_z = ((current_pos.z * 32.0 - last_pos.z * 32.0) * 128.0) as i16;

                let packet = CMoveEntityPos {
                    entity_id,
                    delta_x,
                    delta_y,
                    delta_z,
                    on_ground: true,
                };

                for player in seen_by.values() {
                    player.connection.send_packet(packet.clone());
                }
            } else if rot_changed {
                // Only rotation changed
                let packet = CMoveEntityRot {
                    entity_id,
                    yaw: (current_rot.0 * 256.0 / 360.0) as i8,
                    pitch: (current_rot.1 * 256.0 / 360.0) as i8,
                    on_ground: true,
                };

                for player in seen_by.values() {
                    player.connection.send_packet(packet.clone());
                }
            }

            // Update last position after sending packets
            *last_pos = current_pos;
            *last_rot = current_rot;

            // Send head rotation update separately if yaw changed
            if rot_changed {
                let head_packet = CRotateHead {
                    entity_id: VarInt(entity_id),
                    head_yaw: (current_rot.0 * 256.0 / 360.0) as i8,
                };
                for player in seen_by.values() {
                    player.connection.send_packet(head_packet.clone());
                }
            }
        }

        // Send entity data updates if dirty
        let entity_data = self.entity.entity_data();
        if let Some(dirty_data) = entity_data.pack_dirty() {
            let packet = CSetEntityData {
                entity_id: self.entity.entity_id(),
                metadata: entity_data_to_packet_entries(dirty_data),
            };

            for player in seen_by.values() {
                player.connection.send_packet(packet.clone());
            }
        }
    }

    /// Gets the number of players tracking this entity
    pub fn tracking_player_count(&self) -> usize {
        self.seen_by.read().len()
    }

    /// Broadcasts a packet to all players tracking this entity
    pub fn broadcast_packet<P: steel_protocol::ClientPacket + Clone>(&self, packet: P) {
        let seen_by = self.seen_by.read();
        for player in seen_by.values() {
            player.connection.send_packet(packet.clone());
        }
    }

    /// Gets tracking players (for manual iteration)
    pub fn seen_by_players(&self) -> Vec<Arc<Player>> {
        self.seen_by.read().values().cloned().collect()
    }
}
