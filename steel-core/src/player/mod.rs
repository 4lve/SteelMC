//! This module contains all things player-related.
pub mod chunk_sender;
mod game_profile;
mod message_validator;
/// This module contains the networking implementation for the player.
pub mod networking;
mod signature_cache;

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicI32, Ordering},
};

pub use game_profile::GameProfile;
use message_validator::LastSeenMessagesValidator;
use parking_lot::Mutex;
pub use signature_cache::{LastSeen, MessageCache};

use steel_protocol::packets::{
    common::SCustomPayload,
    game::{CPlayerChat, FilterType, PreviousMessage, SChat, SMovePlayer},
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

    /// Handles a chat message from the player.
    pub fn handle_chat(&self, packet: SChat, player: Arc<Player>) {
        let chat_message = packet.message;
        let sender_index = player
            .messages_sent
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let chat_packet = CPlayerChat::new(
            VarInt(0),
            player.gameprofile.id,
            VarInt(sender_index),
            None,
            chat_message.clone(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
            0,
            Box::new([]),
            Some(TextComponent::new().text(chat_message.clone())),
            FilterType::PassThrough,
            steel_protocol::packets::game::ChatTypeBound {
                registry_id: VarInt(0),
                sender_name: TextComponent::new().text(player.gameprofile.name.clone()),
                target_name: None,
            },
        );

        self.world
            .broadcast_unsigned_chat(chat_packet, &player.gameprofile.name, &chat_message);
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

    /// Cleans up player resources.
    pub fn cleanup(&self) {}
}
