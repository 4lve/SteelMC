//! This module contains all things player-related.
pub mod chunk_sender;
mod game_profile;
pub mod message_chain;
mod message_validator;
/// This module contains the networking implementation for the player.
pub mod networking;
pub mod profile_key;
mod signature_cache;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI32, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub use game_profile::GameProfile;
use message_chain::SignedMessageChain;
use message_validator::LastSeenMessagesValidator;
use parking_lot::Mutex;
use profile_key::RemoteChatSession;
pub use signature_cache::{LastSeen, MessageCache};

use crate::config::STEEL_CONFIG;

use steel_protocol::packets::{
    common::SCustomPayload,
    game::{
        CPlayerChat, FilterType, PreviousMessage, SChat, SChatAck, SChatSessionUpdate, SMovePlayer,
    },
};
use steel_utils::{ChunkPos, codec::VarInt, math::Vector3, text::TextComponent, translations};

/// Re-export `PreviousMessage` as `PreviousMessageEntry` for use in `signature_cache`
pub type PreviousMessageEntry = PreviousMessage;

use crate::{
    chunk::player_chunk_view::PlayerChunkView,
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

    /// The player's position.
    pub position: Mutex<Vector3<f64>>,
    /// The last chunk position of the player.
    pub last_chunk_pos: Mutex<ChunkPos>,
    /// The last chunk tracking view of the player.
    pub last_tracking_view: Mutex<Option<PlayerChunkView>>,
    /// The chunk sender for the player.
    pub chunk_sender: Mutex<ChunkSender>,

    /// Counter for chat messages sent BY this player
    messages_sent: AtomicI32,
    /// Counter for chat messages received BY this player
    messages_received: AtomicI32,

    /// Message signature cache for tracking chat messages
    pub signature_cache: Mutex<MessageCache>,

    /// Validator for client acknowledgements of messages we've sent
    pub message_validator: Mutex<LastSeenMessagesValidator>,

    /// Remote chat session containing the player's public key (if signed chat is enabled)
    pub chat_session: Mutex<Option<RemoteChatSession>>,

    /// Message chain state for tracking signed message sequence
    pub message_chain: Mutex<Option<SignedMessageChain>>,
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
            position: Mutex::new(Vector3::default()),
            last_chunk_pos: Mutex::new(ChunkPos::new(0, 0)),
            last_tracking_view: Mutex::new(None),
            chunk_sender: Mutex::new(ChunkSender::default()),
            messages_sent: AtomicI32::new(0),
            messages_received: AtomicI32::new(0),
            signature_cache: Mutex::new(MessageCache::new()),
            message_validator: Mutex::new(LastSeenMessagesValidator::new()),
            chat_session: Mutex::new(None),
            message_chain: Mutex::new(None),
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

    /// Verifies a signed chat message with comprehensive security checks.
    ///
    /// Returns `Ok(link)` if the signature is valid, or `Err` with a description if invalid.
    ///
    /// Checks performed:
    /// - Chat session exists
    /// - Signature is present
    /// - Key hasn't expired (with grace period)
    /// - Message isn't expired (5 minute window)
    /// - Message chain is valid (ordering, sequence)
    /// - RSA signature is cryptographically valid
    fn verify_chat_signature(
        &self,
        packet: &SChat,
    ) -> Result<message_chain::SignedMessageLink, String> {
        const MESSAGE_EXPIRES_AFTER: Duration = Duration::from_secs(5 * 60);

        // Check if we have a chat session
        let session = self.chat_session.lock().clone().ok_or("No chat session")?;

        // Check if signature is present
        let signature = packet.signature.as_ref().ok_or("No signature present")?;

        // Check if the profile key has expired (with 8-hour grace period)
        if session
            .profile_public_key
            .data()
            .has_expired_with_grace(profile_key::EXPIRY_GRACE_PERIOD)
        {
            return Err("Profile key has expired".to_string());
        }

        // Get the message chain
        let mut chain_guard = self.message_chain.lock();
        let chain = chain_guard.as_mut().ok_or("No message chain")?;

        // Check if chain is broken
        if chain.is_broken() {
            return Err("Message chain is broken".to_string());
        }

        // Convert timestamp from millis to SystemTime
        let timestamp =
            UNIX_EPOCH + Duration::from_millis(packet.timestamp.try_into().unwrap_or(0));

        // Check message expiry (5 minutes as per Minecraft)
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

        // Reconstruct LastSeen from the acknowledged field
        // If we can't reconstruct it (client has state we don't have), use empty LastSeen.
        // This is a workaround for clients that have history from previous sessions.
        // The signature will fail verification, but in non-enforced mode the message still goes through.
        let last_seen = {
            let signature_cache = self.signature_cache.lock();
            signature_cache
                .unpack_acknowledged(packet.message_count, &packet.acknowledged)
                .unwrap_or_else(|| {
                    log::debug!(
                        "Cannot reconstruct LastSeen for {} (offset={}, cache empty). Using empty LastSeen - signature will fail.",
                        self.gameprofile.name,
                        packet.message_count
                    );
                    LastSeen::default()
                })
        };

        let body = message_chain::SignedMessageBody::new(
            packet.message.clone(),
            timestamp,
            packet.salt,
            last_seen,
        );

        // Validate and get the link (this checks ordering and advances the chain)
        let link = chain
            .validate_and_advance(&body)
            .map_err(|e| format!("Chain validation failed: {e}"))?;

        // Create the signature updater
        let updater = message_chain::MessageSignatureUpdater::new(&link, &body);

        // Get the validator from the session's public key
        let validator = session.profile_public_key.create_signature_validator();

        // Verify the signature (call validate on the trait object)
        let is_valid =
            steel_crypto::signature::SignatureValidator::validate(&validator, &updater, signature)
                .map_err(|e| format!("Signature validation error: {e}"))?;

        if is_valid {
            Ok(link)
        } else {
            Err("Invalid signature".to_string())
        }
    }

    /// Handles a chat message from the player.
    #[allow(clippy::too_many_lines)]
    pub fn handle_chat(&self, packet: SChat, player: Arc<Player>) {
        let chat_message = packet.message.clone();

        log::info!(
            "Player {} chat message '{}': has_signature={}, signature_len={}, has_session={}, timestamp={}, salt={}",
            self.gameprofile.name,
            chat_message,
            packet.signature.is_some(),
            packet.signature.as_ref().map_or(0, |s| s.len()),
            self.chat_session.lock().is_some(),
            packet.timestamp,
            packet.salt
        );
        log::info!("Full chat packet: {packet:?}");

        // Try to verify signature if present
        let verification_result = if let Some(signature) = &packet.signature {
            match self.verify_chat_signature(&packet) {
                Ok(link) => {
                    log::info!(
                        "Player {} sent valid signed message (index: {})",
                        self.gameprofile.name,
                        link.index
                    );

                    // Add the signature to the cache for future LastSeen tracking
                    self.signature_cache.lock().add_seen_signature(signature);

                    Some(Ok(link))
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
            log::debug!("Player {} sent unsigned message", self.gameprofile.name);
            None
        };

        // Phase 4: Enforce secure chat if configured
        if STEEL_CONFIG.enforce_secure_chat {
            match &verification_result {
                Some(Ok(_)) => {
                    // Valid signature - proceed
                }
                Some(Err(err)) => {
                    // Invalid signature with enforcement enabled - kick player
                    log::error!(
                        "Player {} kicked for invalid chat signature: {err}",
                        self.gameprofile.name
                    );
                    self.connection.disconnect(
                        TextComponent::new().text(format!("Chat message validation failed: {err}")),
                    );
                    return;
                }
                None => {
                    // No signature with enforcement enabled - kick player
                    log::error!(
                        "Player {} kicked for sending unsigned chat message",
                        self.gameprofile.name
                    );
                    self.connection.disconnect(TextComponent::new().text(
                        "Secure chat is enforced on this server, but your message was not signed",
                    ));
                    return;
                }
            }
        }

        // Determine which signature to use for broadcast
        let signature = if matches!(verification_result, Some(Ok(_))) {
            packet.signature.map(|sig| Box::new(sig) as Box<[u8]>)
        } else {
            None
        };

        let sender_index = player
            .messages_sent
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let chat_packet = CPlayerChat::new(
            VarInt(0),
            player.gameprofile.id,
            VarInt(sender_index),
            signature.clone(),
            chat_message.clone(),
            packet.timestamp,
            packet.salt,
            Box::new([]),
            Some(TextComponent::new().text(chat_message.clone())),
            FilterType::PassThrough,
            steel_protocol::packets::game::ChatTypeBound {
                registry_id: VarInt(0),
                sender_name: TextComponent::new().text(player.gameprofile.name.clone()),
                target_name: None,
            },
        );

        // Use the appropriate broadcast method based on whether we have a valid signature
        if let Some(ref sig_box) = signature {
            // Convert Box<[u8]> to [u8; 128] for broadcast_chat
            if sig_box.len() == 128 {
                let mut sig_array = [0u8; 128];
                sig_array.copy_from_slice(&sig_box[..]);

                // For signed messages, we need to construct the LastSeen
                // For now, use empty LastSeen - full reconstruction will come in Phase 4
                let last_seen = LastSeen::default();

                log::info!("<{}> {}", player.gameprofile.name, chat_message);
                self.world.broadcast_chat(
                    chat_packet,
                    Arc::clone(&player),
                    last_seen,
                    Some(sig_array),
                );
            } else {
                log::warn!(
                    "Player {} signature has wrong length: {}",
                    player.gameprofile.name,
                    sig_box.len()
                );
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

        *self.chat_session.lock() = Some(session);
        *self.message_chain.lock() = Some(chain);

        log::info!(
            "Player {} initialized signed chat session",
            self.gameprofile.name
        );
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

        // Create profile key data
        let profile_key_data =
            profile_key::ProfilePublicKeyData::new(expires_at, public_key, packet.key_signature);

        // For now, we skip Mojang signature validation of the profile key itself
        // The player's key signature should be validated against Mojang's Yggdrasil public key,
        // but since we don't have that hardcoded yet, we'll accept the key as-is
        // TODO: Validate profile key signature against Yggdrasil public keys
        let validator = Box::new(steel_crypto::signature::NoValidation)
            as Box<dyn steel_crypto::SignatureValidator>;

        // Create session data and validate
        let session_data = profile_key::RemoteChatSessionData {
            session_id: packet.session_id,
            profile_public_key: profile_key_data,
        };

        match session_data.validate(self.gameprofile.id, &*validator) {
            Ok(session) => {
                log::info!(
                    "Player {} has valid chat session (expires: {:?})",
                    self.gameprofile.name,
                    session.profile_public_key.data().expires_at
                );
                self.set_chat_session(session);
            }
            Err(err) => {
                log::warn!(
                    "Player {} sent invalid chat session: {err}",
                    self.gameprofile.name
                );
                // Phase 4: Kick if enforcement is enabled
                if STEEL_CONFIG.enforce_secure_chat {
                    log::error!(
                        "Player {} kicked for invalid chat session",
                        self.gameprofile.name
                    );
                    self.connection.disconnect(
                        TextComponent::new().text(format!("Chat session validation failed: {err}")),
                    );
                }
            }
        }
    }

    /// Handles a chat acknowledgment packet from the client.
    ///
    /// This updates the message validator to track which messages the client has seen.
    pub fn handle_chat_ack(&self, packet: SChatAck) {
        // Apply the offset to remove old acknowledged messages
        if let Err(err) = self.message_validator.lock().apply_offset(packet.offset.0) {
            log::warn!(
                "Player {} sent invalid chat acknowledgment: {err}",
                self.gameprofile.name
            );
        } else {
            log::debug!(
                "Player {} acknowledged messages up to offset {}",
                self.gameprofile.name,
                packet.offset.0
            );
        }
    }

    /// Cleans up player resources.
    pub fn cleanup(&self) {}
}
