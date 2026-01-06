//! This module contains the `Server` struct, which is the main entry point for the server.
/// The registry cache for the server.
pub mod registry_cache;
/// The tick rate manager for the server.
pub mod tick_rate_manager;

use std::sync::Arc;
use std::time::Instant;

use steel_crypto::key_store::KeyStore;
use steel_protocol::packets::game::{CLogin, CPlayerPosition, CommonPlayerSpawnInfo};
use steel_registry::{REGISTRY, Registry};
use steel_utils::locks::SyncRwLock;
use steel_utils::{Identifier, types::GameType};
use tick_rate_manager::TickRateManager;
use tokio::runtime::Runtime;
use tokio::task::spawn_blocking;
use tokio_util::sync::CancellationToken;

use crate::command::CommandDispatcher;
use crate::config::STEEL_CONFIG;
use crate::player::Player;
use crate::server::registry_cache::RegistryCache;
use crate::world::World;

/// The main server struct.
pub struct Server {
    /// The cancellation token for graceful shutdown.
    pub cancel_token: CancellationToken,
    /// The key store for the server.
    pub key_store: KeyStore,
    /// The registry cache for the server.
    pub registry_cache: RegistryCache,
    /// A list of all the worlds on the server.
    pub worlds: Vec<Arc<World>>,
    /// The tick rate manager for the server.
    pub tick_rate_manager: SyncRwLock<TickRateManager>,
    /// Saves and dispatches commands to appropriate handlers.
    pub command_dispatcher: SyncRwLock<CommandDispatcher>,
}

impl Server {
    /// Creates a new server.
    ///
    /// # Panics
    ///
    /// Panics if the global registry has already been initialized.
    pub async fn new(chunk_runtime: Arc<Runtime>, cancel_token: CancellationToken) -> Self {
        let start = Instant::now();
        let mut registry = Registry::new_vanilla();
        registry.freeze();
        log::info!("Vanilla registry loaded in {:?}", start.elapsed());

        REGISTRY
            .init(registry)
            .expect("We should be the ones who init the REGISTRY");

        let registry_cache = RegistryCache::new().await;

        Server {
            cancel_token,
            key_store: KeyStore::create(),
            worlds: vec![Arc::new(World::new(chunk_runtime))],
            registry_cache,
            tick_rate_manager: SyncRwLock::new(TickRateManager::new()),
            command_dispatcher: SyncRwLock::new(CommandDispatcher::new()),
        }
    }

    /// Adds a player to the server.
    pub fn add_player(&self, player: Arc<Player>) {
        // Assign entity ID first (before CLogin)
        let entity_id = self.worlds[0].entity_tracker.allocate_entity_id();
        *player.entity_id.lock() = Some(entity_id);

        // Set initial spawn position
        let spawn_pos = steel_utils::math::Vector3::new(0.5, 65.0, 0.5);
        *player.position.lock() = spawn_pos;

        player.connection.send_packet(CLogin {
            player_id: entity_id,
            hardcore: false,
            levels: vec![Identifier::vanilla_static("overworld")],
            max_players: 5,
            chunk_radius: STEEL_CONFIG.view_distance.into(),
            simulation_distance: STEEL_CONFIG.simulation_distance.into(),
            reduced_debug_info: false,
            show_death_screen: false,
            do_limited_crafting: false,
            common_player_spawn_info: CommonPlayerSpawnInfo {
                dimension_type: 0,
                dimension: Identifier::vanilla_static("overworld"),
                seed: 0,
                game_type: GameType::Survival,
                previous_game_type: None,
                is_debug: false,
                is_flat: true,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 64,
            },
            enforces_secure_chat: STEEL_CONFIG.enforce_secure_chat,
        });

        let commands = self.command_dispatcher.read().get_commands();
        player.connection.send_packet(commands);

        // Send the player their spawn position
        let teleport_id = 1; // Start with ID 1 (0 might be ignored by some clients)
        player.set_awaiting_teleport(spawn_pos, teleport_id);
        player.connection.send_packet(CPlayerPosition {
            x: spawn_pos.x,
            y: spawn_pos.y,
            z: spawn_pos.z,
            delta_x: 0.0,
            delta_y: 0.0,
            delta_z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            relatives: 0, // All absolute
            teleport_id,
        });

        self.worlds[0].add_player(player);
    }

    /// Runs the server tick loop.
    pub async fn run(self: Arc<Self>, cancel_token: CancellationToken) {
        let mut next_tick_time = Instant::now();

        loop {
            if cancel_token.is_cancelled() {
                break;
            }

            let (nanoseconds_per_tick, is_sprinting, should_sprint_this_tick) = {
                let mut tick_manager = self.tick_rate_manager.write();
                let nanoseconds_per_tick = tick_manager.nanoseconds_per_tick;

                // Handle sprinting
                let is_sprinting = tick_manager.is_sprinting();
                let should_sprint_this_tick = if is_sprinting {
                    tick_manager.check_should_sprint_this_tick()
                } else {
                    false
                };
                (nanoseconds_per_tick, is_sprinting, should_sprint_this_tick)
            };

            if is_sprinting && should_sprint_this_tick {
                // If sprinting, we don't wait
                next_tick_time = Instant::now();
            } else {
                // Normal wait logic
                let now = Instant::now();
                if now < next_tick_time {
                    tokio::select! {
                        () = cancel_token.cancelled() => break,
                        () = tokio::time::sleep(next_tick_time - now) => {}
                    }
                }
                next_tick_time += std::time::Duration::from_nanos(nanoseconds_per_tick);
            }

            if cancel_token.is_cancelled() {
                break;
            }

            let tick_count = {
                let mut tick_manager = self.tick_rate_manager.write();
                tick_manager.tick();
                tick_manager.tick_count
            };

            // Tick worlds
            self.tick_worlds(tick_count).await;

            if is_sprinting && should_sprint_this_tick {
                let mut tick_manager = self.tick_rate_manager.write();
                tick_manager.end_tick_work();
            }
        }
    }

    async fn tick_worlds(&self, tick_count: u64) {
        let mut tasks = Vec::with_capacity(self.worlds.len());
        for world in &self.worlds {
            let world_clone = world.clone();
            tasks.push(spawn_blocking(move || world_clone.tick_b(tick_count)));
        }
        let start = Instant::now();
        for task in tasks {
            let _ = task.await;
        }
        if start.elapsed().as_millis() > 1 {
            log::warn!(
                "Worlds ticked in {:?}, tick count: {tick_count}",
                start.elapsed()
            );
        }
    }
}
