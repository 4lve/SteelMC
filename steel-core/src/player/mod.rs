//! This module contains all things player-related.
pub mod chunk_sender;
mod game_profile;
pub mod message_chain;
mod message_validator;
/// This module contains the networking implementation for the player.
pub mod networking;
pub mod player_inventory;
pub mod profile_key;
mod signature_cache;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossbeam::atomic::AtomicCell;
pub use game_profile::GameProfile;
use message_chain::SignedMessageChain;
use message_validator::LastSeenMessagesValidator;
use profile_key::RemoteChatSession;
pub use signature_cache::{LastSeen, MessageCache};
use steel_utils::locks::SyncMutex;
use steel_utils::types::GameType;

use crate::config::STEEL_CONFIG;
use crate::player::player_inventory::PlayerInventory;

use steel_protocol::packets::{
    common::SCustomPayload,
    game::{
        CAnimate, CPlayerChat, FilterType, InteractionHand, PlayerCommandAction, PreviousMessage,
        SChat, SChatAck, SChatSessionUpdate, SContainerButtonClick, SContainerClick,
        SContainerClose, SContainerSlotStateChanged, SMovePlayer, SSetCreativeModeSlot,
    },
};
use steel_utils::{ChunkPos, math::Vector3, text::TextComponent, translations};

use crate::entity::LivingEntity;
use crate::inventory::{
    SyncContainer,
    container::{Container, ContainerType},
    equipment::{EntityEquipment, EquipmentSlot},
    inventory_menu::InventoryMenu,
    menu::Menu,
    slot::Slot,
};

/// Re-export `PreviousMessage` as `PreviousMessageEntry` for use in `signature_cache`
pub type PreviousMessageEntry = PreviousMessage;

use crate::{
    chunk::player_chunk_view::PlayerChunkView,
    entity::{EntityDataAccessor, Pose},
    player::{chunk_sender::ChunkSender, networking::JavaConnection},
    world::World,
};

/// A struct representing a player.
pub struct Player {
    /// The player's game profile.
    pub gameprofile: GameProfile,
    /// The player's connection.
    pub connection: Arc<JavaConnection>,

    /// The world the player is in.
    pub world: Arc<World>,

    /// Whether the player has finished loading the client.
    pub client_loaded: AtomicBool,

    /// The player's entity ID (for entity tracking)
    pub entity_id: SyncMutex<Option<i32>>,

    /// The player's position.
    pub position: SyncMutex<Vector3<f64>>,
    /// The player's rotation (yaw, pitch).
    pub rotation: SyncMutex<(f32, f32)>,
    /// The player's pose (standing, crouching, etc.).
    pub pose: SyncMutex<Pose>,

    // LivingEntity fields
    /// The player's health (synced with client via entity data).
    health: AtomicCell<f32>,
    /// The player's absorption amount (extra health from effects like Absorption).
    absorption_amount: AtomicCell<f32>,
    /// The player's movement speed.
    speed: AtomicCell<f32>,
    /// Whether the player is sprinting.
    sprinting: AtomicBool,

    /// The last chunk position of the player.
    pub last_chunk_pos: SyncMutex<ChunkPos>,
    /// The last chunk tracking view of the player.
    pub last_tracking_view: SyncMutex<Option<PlayerChunkView>>,
    /// The chunk sender for the player.
    pub chunk_sender: SyncMutex<ChunkSender>,

    /// Teleport tracking - position we're waiting for client to acknowledge
    awaiting_position_from_client: SyncMutex<Option<Vector3<f64>>>,
    /// Teleport ID we're waiting for
    awaiting_teleport_id: AtomicI32,

    /// Counter for chat messages sent BY this player
    messages_sent: AtomicI32,
    /// Counter for chat messages received BY this player
    messages_received: AtomicI32,

    /// Message signature cache for tracking chat messages
    pub signature_cache: SyncMutex<MessageCache>,

    /// Validator for client acknowledgements of messages we've sent
    pub message_validator: SyncMutex<LastSeenMessagesValidator>,

    /// Remote chat session containing the player's public key (if signed chat is enabled)
    pub chat_session: SyncMutex<Option<RemoteChatSession>>,

    /// Message chain state for tracking signed message sequence
    pub message_chain: SyncMutex<Option<SignedMessageChain>>,

    /// The player's current game mode (Survival, Creative, Adventure, Spectator)
    pub game_mode: AtomicCell<GameType>,

    /// Entity equipment (armor, offhand, hands).
    /// `MainHand` will delegate to inventory when inventory is implemented.
    equipment: Arc<SyncMutex<EntityEquipment>>,

    /// The player's inventory container (shared with `inventory_menu`).
    pub inventory: SyncContainer,

    /// The player's inventory menu (always open, even when `container_id` is 0).
    inventory_menu: SyncMutex<InventoryMenu>,
}

impl Player {
    /// Creates a new player.
    pub fn new(
        gameprofile: GameProfile,
        connection: Arc<JavaConnection>,
        world: Arc<World>,
        player: &std::sync::Weak<Player>,
    ) -> Self {
        let entity_equipment = Arc::new(SyncMutex::new(EntityEquipment::new()));

        // Create a single shared inventory container used by both the player and inventory menu
        let inventory: SyncContainer = Arc::new(SyncMutex::new(ContainerType::PlayerInventory(
            PlayerInventory::new(entity_equipment.clone(), player.clone()),
        )));

        Self {
            gameprofile,
            connection,

            world,
            client_loaded: AtomicBool::new(false),
            entity_id: SyncMutex::new(None),
            position: SyncMutex::new(Vector3::default()),
            rotation: SyncMutex::new((0.0, 0.0)),
            pose: SyncMutex::new(Pose::Standing),
            health: AtomicCell::new(20.0), // Default max health
            absorption_amount: AtomicCell::new(0.0),
            speed: AtomicCell::new(0.1), // Default walking speed
            sprinting: AtomicBool::new(false),
            last_chunk_pos: SyncMutex::new(ChunkPos::new(0, 0)),
            last_tracking_view: SyncMutex::new(None),
            chunk_sender: SyncMutex::new(ChunkSender::default()),
            awaiting_position_from_client: SyncMutex::new(None),
            awaiting_teleport_id: AtomicI32::new(0),
            messages_sent: AtomicI32::new(0),
            messages_received: AtomicI32::new(0),
            signature_cache: SyncMutex::new(MessageCache::new()),
            message_validator: SyncMutex::new(LastSeenMessagesValidator::new()),
            chat_session: SyncMutex::new(None),
            message_chain: SyncMutex::new(None),
            game_mode: AtomicCell::new(GameType::Survival),
            equipment: entity_equipment.clone(),
            inventory: inventory.clone(),
            inventory_menu: SyncMutex::new(InventoryMenu::new(inventory)),
        }
    }

    /// Ticks the player.
    #[allow(clippy::cast_possible_truncation)]
    pub fn tick(&self) {
        if !self.client_loaded.load(Ordering::Relaxed) {
            //return;
        }

        let current_pos = *self.position.lock();
        let chunk_x = (current_pos.x as i32) >> 4;
        let chunk_z = (current_pos.z as i32) >> 4;
        let chunk_pos = ChunkPos::new(chunk_x, chunk_z);

        *self.last_chunk_pos.lock() = chunk_pos;

        self.world.chunk_map.update_player_status(self);

        self.chunk_sender
            .lock()
            .send_next_chunks(self.connection.clone(), &self.world, chunk_pos);

        // Broadcast inventory changes to client
        self.broadcast_inventory_changes();

        self.connection.tick();

        // TODO: Implement player ticking logic here
        // This will include:
        // - Checking if the player is alive
        // - Handling movement
        // - Updating inventory
        // - Handling food/health regeneration
        // - Managing game mode specific logic
        // - Updating advancements
        // - Handling falling
    }

    /// Handles a custom payload packet.
    pub fn handle_custom_payload(&self, packet: SCustomPayload) {
        log::info!("Hello from the other side! {packet:?}");
    }

    /// Handles the end of a client tick.
    pub fn handle_client_tick_end(&self) {
        //log::info!("Hello from the other side!");
    }

    /// Gets the next `messages_received` counter and increments it
    pub fn get_and_increment_messages_received(&self) -> i32 {
        self.messages_received.fetch_add(1, Ordering::Relaxed)
    }

    fn verify_chat_signature(
        &self,
        packet: &SChat,
    ) -> Result<(message_chain::SignedMessageLink, LastSeen), String> {
        const MESSAGE_EXPIRES_AFTER: Duration = Duration::from_secs(5 * 60);

        let session = self.chat_session.lock().clone().ok_or("No chat session")?;
        let signature = packet.signature.as_ref().ok_or("No signature present")?;

        if session
            .profile_public_key
            .data()
            .has_expired_with_grace(profile_key::EXPIRY_GRACE_PERIOD)
        {
            return Err("Profile key has expired".to_string());
        }

        let mut chain_guard = self.message_chain.lock();
        let chain = chain_guard.as_mut().ok_or("No message chain")?;

        if chain.is_broken() {
            return Err("Message chain is broken".to_string());
        }

        let timestamp =
            UNIX_EPOCH + Duration::from_millis(packet.timestamp.try_into().unwrap_or(0));

        let now = SystemTime::now();
        let message_age = now
            .duration_since(timestamp)
            .unwrap_or(Duration::from_secs(0));

        if message_age > MESSAGE_EXPIRES_AFTER {
            return Err(format!(
                "Message expired (age: {}s, max: 300s)",
                message_age.as_secs()
            ));
        }

        let last_seen_signatures = self
            .message_validator
            .lock()
            .apply_update(packet.acknowledged, packet.offset, packet.checksum)
            .map_err(|e| {
                log::error!("Message acknowledgment validation failed: {e}");
                e
            })?;

        let last_seen = LastSeen::new(last_seen_signatures);

        let body = message_chain::SignedMessageBody::new(
            packet.message.clone(),
            timestamp,
            packet.salt,
            last_seen,
        );

        let link = chain
            .validate_and_advance(&body)
            .map_err(|e| format!("Chain validation failed: {e}"))?;

        let updater = message_chain::MessageSignatureUpdater::new(&link, &body);
        let validator = session.profile_public_key.create_signature_validator();

        let is_valid =
            steel_crypto::signature::SignatureValidator::validate(&validator, &updater, signature)
                .map_err(|e| format!("Signature validation error: {e}"))?;

        if is_valid {
            Ok((link, body.last_seen.clone()))
        } else {
            Err("Invalid signature".to_string())
        }
    }

    /// Handles a chat message from the player.
    #[allow(clippy::too_many_lines)]
    pub fn handle_chat(&self, packet: SChat, player: Arc<Player>) {
        let chat_message = packet.message.clone();

        let verification_result = if let Some(_signature) = &packet.signature {
            match self.verify_chat_signature(&packet) {
                Ok((link, last_seen)) => {
                    // Don't add to cache here - will be added during broadcast
                    // to avoid cache state mismatch with client
                    Some(Ok((link, last_seen)))
                }
                Err(err) => {
                    log::warn!(
                        "Player {} sent message with invalid signature: {err}",
                        self.gameprofile.name
                    );
                    Some(Err(err))
                }
            }
        } else {
            None
        };

        if STEEL_CONFIG.enforce_secure_chat {
            match &verification_result {
                Some(Ok(_)) => {}
                Some(Err(err)) => {
                    self.connection.disconnect(
                        TextComponent::new().text(format!("Chat message validation failed: {err}")),
                    );
                    return;
                }
                None => {
                    self.connection.disconnect(TextComponent::new().text(
                        "Secure chat is enforced on this server, but your message was not signed",
                    ));
                    return;
                }
            }
        }

        let signature = if matches!(verification_result, Some(Ok(_))) {
            packet.signature.map(|sig| Box::new(sig) as Box<[u8]>)
        } else {
            None
        };

        let sender_index = player
            .messages_sent
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let chat_packet = CPlayerChat::new(
            0,
            player.gameprofile.id,
            sender_index,
            signature.clone(),
            chat_message.clone(),
            packet.timestamp,
            packet.salt,
            Box::new([]),
            Some(TextComponent::new().text(chat_message.clone())),
            FilterType::PassThrough,
            steel_protocol::packets::game::ChatTypeBound {
                //TODO: Use the registry to derive this instead of hardcoding it
                registry_id: 0,
                sender_name: TextComponent::new().text(player.gameprofile.name.clone()),
                target_name: None,
            },
        );

        if let Some(ref sig_box) = signature {
            if sig_box.len() == 256 {
                let mut sig_array = [0u8; 256];
                sig_array.copy_from_slice(&sig_box[..]);

                let last_seen = if let Some(Ok((_, ref last_seen))) = verification_result {
                    last_seen.clone()
                } else {
                    LastSeen::default()
                };

                log::info!("<{}> {}", player.gameprofile.name, chat_message);
                self.world.broadcast_chat(
                    chat_packet,
                    Arc::clone(&player),
                    last_seen,
                    Some(sig_array),
                );
            } else {
                self.world.broadcast_unsigned_chat(
                    chat_packet,
                    &player.gameprofile.name,
                    &chat_message,
                );
            }
        } else {
            self.world.broadcast_unsigned_chat(
                chat_packet,
                &player.gameprofile.name,
                &chat_message,
            );
        }
    }

    fn is_invalid_position(x: f64, y: f64, z: f64, rot_x: f32, rot_y: f32) -> bool {
        if x.is_nan() || y.is_nan() || z.is_nan() {
            return true;
        }

        if !rot_x.is_finite() || !rot_y.is_finite() {
            return true;
        }

        false
    }

    fn update_awaiting_teleport(&self) -> bool {
        self.awaiting_position_from_client.lock().is_some()
    }

    /// Sets the awaiting teleport position (called when sending `CPlayerPosition`)
    pub fn set_awaiting_teleport(&self, position: Vector3<f64>, teleport_id: i32) {
        *self.awaiting_position_from_client.lock() = Some(position);
        self.awaiting_teleport_id
            .store(teleport_id, Ordering::Relaxed);
    }

    /// Handles teleport acknowledgment from client
    pub fn handle_accept_teleportation(&self, teleport_id: i32) {
        let expected_id = self.awaiting_teleport_id.load(Ordering::Relaxed);
        if teleport_id == expected_id
            && let Some(pos) = self.awaiting_position_from_client.lock().take()
        {
            *self.position.lock() = pos;
        }
    }

    /// Handles a player input packet (movement keys, sneak flag).
    pub fn handle_player_input(&self, packet: steel_protocol::packets::game::SPlayerInput) {
        // Update player pose based on sneak flag
        let new_pose = if packet.flags.sneak {
            if matches!(*self.pose.lock(), Pose::Crouching) {
                None
            } else {
                *self.pose.lock() = Pose::Crouching;
                Some(Pose::Crouching)
            }
        } else {
            // Not sneaking - return to standing (unless swimming/flying/etc.)
            if matches!(*self.pose.lock(), Pose::Crouching) {
                *self.pose.lock() = Pose::Standing;
                Some(Pose::Standing)
            } else {
                None
            }
        };

        // Update entity metadata if pose changed
        if let Some(pose) = new_pose
            && let Some(entity_id) = *self.entity_id.lock()
            && let Some(tracked_entity) = self.world.entity_tracker.get_entity(entity_id)
        {
            tracked_entity
                .entity
                .entity_data()
                .set(EntityDataAccessor::POSE, pose);
        }
    }

    /// Handles a player command packet (sprinting, elytra flying, etc.).
    pub fn handle_player_command(&self, packet: steel_protocol::packets::game::SPlayerCommand) {
        // Update player pose based on command
        let new_pose = match packet.action {
            PlayerCommandAction::StartFlyingWithElytra => {
                *self.pose.lock() = Pose::FallFlying;
                Some(Pose::FallFlying)
            }
            PlayerCommandAction::StartSprinting | PlayerCommandAction::StopSprinting => {
                // Sprinting doesn't change pose, just movement speed
                None
            }
            _ => None,
        };

        // Update entity metadata if pose changed
        if let Some(pose) = new_pose
            && let Some(entity_id) = *self.entity_id.lock()
            && let Some(tracked_entity) = self.world.entity_tracker.get_entity(entity_id)
        {
            tracked_entity
                .entity
                .entity_data()
                .set(EntityDataAccessor::POSE, pose);
        }
    }

    /// Handles a swing packet (player arm animation).
    pub fn handle_swing(&self, packet: steel_protocol::packets::game::SSwing) {
        // Get the player's entity ID
        let Some(entity_id) = *self.entity_id.lock() else {
            log::warn!(
                "Player {} tried to swing but has no entity ID",
                self.gameprofile.name
            );
            return;
        };

        // Get the tracked entity from the entity tracker
        let Some(tracked_entity) = self.world.entity_tracker.get_entity(entity_id) else {
            log::warn!(
                "Player {} tried to swing but entity {} not found in tracker",
                self.gameprofile.name,
                entity_id
            );
            return;
        };

        // Create animation packet based on which hand is swinging
        let animation_packet = match packet.hand {
            InteractionHand::MainHand => CAnimate::swing_main_hand(entity_id),
            InteractionHand::OffHand => CAnimate::swing_offhand(entity_id),
        };

        // Broadcast to all players who can see this entity
        tracked_entity.broadcast_packet(animation_packet);
    }

    /// Wraps degrees to -180 to 180 range (matches Minecraft's Mth.wrapDegrees)
    fn wrap_degrees(value: f32) -> f32 {
        let mut normalized = value % 360.0;
        if normalized >= 180.0 {
            normalized -= 360.0;
        }
        if normalized < -180.0 {
            normalized += 360.0;
        }
        normalized
    }

    /// Handles a move player packet.
    pub fn handle_move_player(&self, packet: SMovePlayer) {
        if Self::is_invalid_position(
            packet.get_x(0.0),
            packet.get_y(0.0),
            packet.get_z(0.0),
            packet.get_x_rot(0.0),
            packet.get_y_rot(0.0),
        ) {
            self.connection
                .disconnect(translations::MULTIPLAYER_DISCONNECT_INVALID_PLAYER_MOVEMENT.msg());
            return;
        }

        if !self.update_awaiting_teleport() && self.client_loaded.load(Ordering::Relaxed) {
            if packet.has_pos {
                *self.position.lock() = packet.position;
            }
            if packet.has_rot {
                // Normalize rotation to -180 to 180 range (matches Minecraft's Mth.wrapDegrees)
                let yaw = Self::wrap_degrees(packet.y_rot);
                let pitch = Self::wrap_degrees(packet.x_rot);
                *self.rotation.lock() = (yaw, pitch);
            }
        }
    }

    /// Updates the player's chat session and initializes the message chain.
    ///
    /// This should be called when receiving a `ChatSessionUpdate` packet from the client.
    pub fn set_chat_session(&self, session: RemoteChatSession) {
        // Initialize the message chain for this session
        let chain = SignedMessageChain::new(self.gameprofile.id, session.session_id);

        // Convert session to data for broadcasting
        let session_data = session.as_data();
        let protocol_data = match session_data.to_protocol_data() {
            Ok(data) => data,
            Err(err) => {
                log::error!(
                    "Failed to convert chat session to protocol data for {}: {:?}",
                    self.gameprofile.name,
                    err
                );
                *self.chat_session.lock() = Some(session);
                *self.message_chain.lock() = Some(chain);
                return;
            }
        };

        *self.chat_session.lock() = Some(session);
        *self.message_chain.lock() = Some(chain);

        log::info!(
            "Player {} initialized signed chat session",
            self.gameprofile.name
        );

        // Broadcast the chat session to all players so they can verify this player's signatures
        let update_packet = steel_protocol::packets::game::CPlayerInfoUpdate::update_chat_session(
            self.gameprofile.id,
            protocol_data,
        );

        self.world.players.iter_sync(|_, player| {
            player.connection.send_packet(update_packet.clone());
            true
        });
    }

    /// Gets a reference to the player's chat session if present
    pub fn chat_session(&self) -> Option<RemoteChatSession> {
        self.chat_session.lock().clone()
    }

    /// Checks if the player has a valid chat session
    pub fn has_chat_session(&self) -> bool {
        self.chat_session.lock().is_some()
    }

    /// Handles a chat session update packet from the client.
    ///
    /// This validates the player's profile key and initializes signed chat if valid.
    pub fn handle_chat_session_update(&self, packet: SChatSessionUpdate) {
        log::info!("Player {} sent chat session update", self.gameprofile.name);

        // Convert the packet data to profile key data
        let expires_at = UNIX_EPOCH + Duration::from_millis(packet.expires_at as u64);

        // Decode the public key
        let public_key = match steel_crypto::public_key_from_bytes(&packet.public_key) {
            Ok(key) => key,
            Err(err) => {
                log::warn!(
                    "Player {} sent invalid public key: {err}",
                    self.gameprofile.name
                );
                // Phase 4: Kick if enforcement is enabled
                if STEEL_CONFIG.enforce_secure_chat {
                    log::error!(
                        "Player {} kicked for invalid public key",
                        self.gameprofile.name
                    );
                    self.connection
                        .disconnect(TextComponent::new().text("Invalid profile public key"));
                }
                return;
            }
        };

        let profile_key_data =
            profile_key::ProfilePublicKeyData::new(expires_at, public_key, packet.key_signature);

        let validator = Box::new(steel_crypto::signature::NoValidation)
            as Box<dyn steel_crypto::SignatureValidator>;

        let session_data = profile_key::RemoteChatSessionData {
            session_id: packet.session_id,
            profile_public_key: profile_key_data,
        };

        match session_data.validate(self.gameprofile.id, &*validator) {
            Ok(session) => {
                self.set_chat_session(session);
            }
            Err(err) => {
                log::warn!(
                    "Player {} sent invalid chat session: {err}",
                    self.gameprofile.name
                );
                if STEEL_CONFIG.enforce_secure_chat {
                    self.connection.disconnect(
                        TextComponent::new().text(format!("Chat session validation failed: {err}")),
                    );
                }
            }
        }
    }

    /// Handles a chat acknowledgment packet from the client.
    pub fn handle_chat_ack(&self, packet: SChatAck) {
        if let Err(err) = self.message_validator.lock().apply_offset(packet.offset.0) {
            log::warn!(
                "Player {} sent invalid chat acknowledgment: {err}",
                self.gameprofile.name
            );
        }
    }

    /// Sets the player's game mode and notifies the client.
    ///
    /// Returns `true` if the game mode was changed, `false` if the player was already in the requested game mode.
    pub fn set_game_mode(&self, gamemode: steel_utils::types::GameType) -> bool {
        use steel_protocol::packets::game::{CGameEvent, GameEventType};

        let current_gamemode = self.game_mode.load();
        if current_gamemode == gamemode {
            return false;
        }

        self.game_mode.store(gamemode);

        self.connection.send_packet(CGameEvent {
            event: GameEventType::ChangeGameMode,
            data: gamemode.into(),
        });

        true
    }

    /// Handles a container button click packet (e.g., enchanting table buttons).
    pub fn handle_container_button_click(&self, packet: SContainerButtonClick) {
        log::debug!(
            "Player {} clicked button {} in container {}",
            self.gameprofile.name,
            packet.button_id,
            packet.container_id
        );
        // TODO: Implement container button click handling
        // This is used for things like:
        // - Enchanting table level selection
        // - Stonecutter recipe selection
        // - Loom pattern selection
        // - Lectern page turning
    }

    /// Handles a container click packet (slot interaction).
    pub fn handle_container_click(&self, packet: SContainerClick) {
        let mut menu = self.inventory_menu.lock();

        // Check container ID matches
        if i32::from(menu.behavior().container_id) != packet.container_id {
            return;
        }

        // Handle spectator mode - just resync the inventory
        if self.game_mode.load() == GameType::Spectator {
            menu.behavior_mut()
                .send_all_data_to_remote(&self.connection);
            return;
        }

        // Validate slot index
        if !menu.behavior().is_valid_slot_index(packet.slot_num) {
            log::debug!(
                "Player {} clicked invalid slot index: {}, available: {}",
                self.gameprofile.name,
                packet.slot_num,
                menu.behavior().slot_count()
            );
            return;
        }

        // Check if we need a full resync (state ID mismatch)
        let full_resync_needed = packet.state_id as u32 != menu.behavior().get_state_id();

        // Suppress remote updates during click handling
        menu.behavior_mut().suppress_remote_updates();

        // Handle the click using the Menu trait method
        let has_infinite_materials = self.game_mode.load() == GameType::Creative;
        menu.clicked(
            packet.slot_num,
            packet.button_num,
            packet.click_type,
            has_infinite_materials,
            self,
        );

        // Update remote slots from the client's perception
        for (slot, hash) in packet.changed_slots {
            menu.behavior_mut().set_remote_slot(slot as usize, hash);
        }

        // Update remote carried from the client's perception
        menu.behavior_mut().set_remote_carried(packet.carried_item);

        // Resume remote updates
        menu.behavior_mut().resume_remote_updates();

        // Broadcast changes or full state depending on whether we had a state mismatch
        if full_resync_needed {
            menu.behavior_mut().broadcast_full_state(&self.connection);
        } else {
            menu.behavior_mut().broadcast_changes(&self.connection);
        }
    }

    /// Handles a container close packet.
    pub fn handle_container_close(&self, packet: SContainerClose) {
        log::debug!(
            "Player {} closed container {}",
            self.gameprofile.name,
            packet.container_id
        );

        // For the player inventory menu (container_id 0), call removed() to:
        // - Return crafting grid items to inventory
        // - Clear the cursor item
        if packet.container_id == i32::from(InventoryMenu::CONTAINER_ID) {
            let mut menu = self.inventory_menu.lock();
            menu.removed(self);
        }
        // TODO: Handle other container types (chests, crafting tables, etc.)
        // - Notify any block entities that the player left
    }

    /// Handles a container slot state changed packet (e.g., crafter slot toggle).
    pub fn handle_container_slot_state_changed(&self, packet: SContainerSlotStateChanged) {
        log::debug!(
            "Player {} changed slot {} state to {} in container {}",
            self.gameprofile.name,
            packet.slot_id,
            packet.new_state,
            packet.container_id
        );
        // TODO: Implement slot state change handling
        // This is used for the crafter block to enable/disable slots
    }

    /// Handles a creative mode slot set packet.
    pub fn handle_set_creative_mode_slot(&self, packet: SSetCreativeModeSlot) {
        // Only allow in creative mode
        if self.game_mode.load() != GameType::Creative {
            return;
        }

        let drop = packet.slot_num < 0;
        let item_stack = packet.item_stack;

        // Validate slot range (1-45 for inventory menu)
        let valid_slot = packet.slot_num >= 1 && packet.slot_num <= 45;
        let valid_data = item_stack.is_empty() || item_stack.count <= item_stack.max_stack_size();

        if valid_slot && valid_data {
            let mut menu = self.inventory_menu.lock();
            let slot_index = packet.slot_num as usize;

            if let Some(slot) = menu.behavior().get_slot(slot_index) {
                slot.set_item(item_stack.clone());
            }

            menu.behavior_mut()
                .set_remote_slot_known(slot_index, &item_stack);
            menu.behavior_mut().broadcast_changes(&self.connection);
        } else if drop && valid_data {
            // TODO: Implement drop spam throttling
            // For now, just drop the item
            if !item_stack.is_empty() {
                // TODO: Actually drop the item into the world
                log::debug!(
                    "Player {} would drop {:?} in creative mode",
                    self.gameprofile.name,
                    item_stack
                );
            }
        }
    }

    /// Sends all inventory slots to the client (full sync).
    /// This should be called when the player first joins.
    pub fn send_inventory_to_remote(&self) {
        self.inventory_menu
            .lock()
            .behavior_mut()
            .send_all_data_to_remote(&self.connection);
    }

    /// Broadcasts inventory changes to the client (incremental sync).
    /// This is called every tick to sync only changed slots.
    pub fn broadcast_inventory_changes(&self) {
        self.inventory_menu
            .lock()
            .behavior_mut()
            .broadcast_changes(&self.connection);
    }

    /// Drops an item into the world.
    ///
    /// Based on Java's `Player.drop(ItemStack, boolean throwRandomly)`.
    ///
    /// - `throw_randomly`: If true, the item is thrown in a random direction (like pressing Q).
    ///   If false, it's thrown in the direction the player is facing.
    pub fn drop_item(&self, item: steel_registry::item_stack::ItemStack, throw_randomly: bool) {
        if item.is_empty() {
            return;
        }
        // TODO: Spawn an ItemEntity in the world at the player's position
        // For now, just log it
        log::debug!(
            "Player {} dropped item: {:?} (throw_randomly: {})",
            self.gameprofile.name,
            item,
            throw_randomly
        );
    }

    /// Returns true if the player can drop items.
    ///
    /// Based on Java's `Player.canDropItems()`.
    /// Returns false if the player is dead, removed, or has a flag preventing item drops.
    #[must_use]
    pub fn can_drop_items(&self) -> bool {
        // TODO: Check if player is alive and not removed
        // For now, always return true
        true
    }

    /// Tries to add an item to the player's inventory, dropping it if it doesn't fit.
    ///
    /// Based on Java's `Inventory.placeItemBackInInventory`.
    pub fn add_item_or_drop(&self, mut item: steel_registry::item_stack::ItemStack) {
        if item.is_empty() {
            return;
        }

        // Try to add to inventory
        let added = self.inventory.lock().add(&mut item);
        if !added || !item.is_empty() {
            // Couldn't fit everything, drop the rest
            self.drop_item(item, false);
        }
    }

    /// Cleans up player resources.
    pub fn cleanup(&self) {}
}

impl LivingEntity for Player {
    fn get_health(&self) -> f32 {
        self.health.load()
    }

    fn set_health(&mut self, health: f32) {
        let max_health = self.get_max_health();
        let clamped = health.clamp(0.0, max_health);
        self.health.store(clamped);
        // TODO: Sync health to client via entity data
    }

    fn get_max_health(&self) -> f32 {
        // TODO: Get from attributes system when implemented
        20.0
    }

    fn get_position(&self) -> Vector3<f64> {
        *self.position.lock()
    }

    fn get_absorption_amount(&self) -> f32 {
        self.absorption_amount.load()
    }

    fn set_absorption_amount(&mut self, amount: f32) {
        self.absorption_amount.store(amount.max(0.0));
        // TODO: Sync to client
    }

    fn get_armor_value(&self) -> i32 {
        // TODO: Calculate from equipped items when data components are implemented
        // Will iterate over ARMOR_SLOTS and sum armor values from each piece
        0
    }

    fn get_item_by_slot(&self, slot: EquipmentSlot) -> steel_registry::item_stack::ItemStack {
        self.equipment.lock().get_cloned(slot)
    }

    fn is_sprinting(&self) -> bool {
        self.sprinting.load(Ordering::Relaxed)
    }

    fn set_sprinting(&mut self, sprinting: bool) {
        self.sprinting.store(sprinting, Ordering::Relaxed);
        // TODO: Apply speed modifiers when attribute system is implemented
    }

    fn get_speed(&self) -> f32 {
        self.speed.load()
    }

    fn set_speed(&mut self, speed: f32) {
        self.speed.store(speed);
    }
}
