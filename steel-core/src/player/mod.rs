//! This module contains all things player-related.
pub mod chunk_sender;
mod game_profile;
pub mod message_chain;
mod message_validator;
/// This module contains the networking implementation for the player.
pub mod networking;
pub mod profile_key;
mod signature_cache;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub use game_profile::GameProfile;
use message_chain::SignedMessageChain;
use message_validator::LastSeenMessagesValidator;
use profile_key::RemoteChatSession;
pub use signature_cache::{LastSeen, MessageCache};
use steel_utils::locks::SyncMutex;

use crate::config::STEEL_CONFIG;

use steel_protocol::packets::{
    common::SCustomPayload,
    game::{
        CPlayerChat, FilterType, PreviousMessage, SChat, SChatAck, SChatSessionUpdate, SMovePlayer,
    },
};
use steel_utils::{ChunkPos, math::Vector3, text::TextComponent, translations};

/// Re-export `PreviousMessage` as `PreviousMessageEntry` for use in `signature_cache`
pub type PreviousMessageEntry = PreviousMessage;

use crate::chunk::player_chunk_view::PlayerChunkView;
use crate::player::{chunk_sender::ChunkSender, networking::JavaConnection};
use crate::world::World;

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

    /// The player's position.
    pub position: SyncMutex<Vector3<f64>>,
    /// The last chunk position of the player.
    pub last_chunk_pos: SyncMutex<ChunkPos>,
    /// The last chunk tracking view of the player.
    pub last_tracking_view: SyncMutex<Option<PlayerChunkView>>,
    /// The chunk sender for the player.
    pub chunk_sender: SyncMutex<ChunkSender>,

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
}

impl Player {
    /// Creates a new player.
    pub fn new(
        gameprofile: GameProfile,
        connection: Arc<JavaConnection>,
        world: Arc<World>,
    ) -> Self {
        Self {
            gameprofile,
            connection,

            world,
            client_loaded: AtomicBool::new(false),
            position: SyncMutex::new(Vector3::default()),
            last_chunk_pos: SyncMutex::new(ChunkPos::new(0, 0)),
            last_tracking_view: SyncMutex::new(None),
            chunk_sender: SyncMutex::new(ChunkSender::default()),
            messages_sent: AtomicI32::new(0),
            messages_received: AtomicI32::new(0),
            signature_cache: SyncMutex::new(MessageCache::new()),
            message_validator: SyncMutex::new(LastSeenMessagesValidator::new()),
            chat_session: SyncMutex::new(None),
            message_chain: SyncMutex::new(None),
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

    #[allow(clippy::unused_self)]
    fn update_awaiting_teleport(&self) -> bool {
        //TODO: Implement this
        false
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

        if !self.update_awaiting_teleport()
            && self.client_loaded.load(Ordering::Relaxed)
            && packet.has_pos
        {
            *self.position.lock() = packet.position;
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

    /// Cleans up player resources.
    pub fn cleanup(&self) {}
}
