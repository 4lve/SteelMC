//! This module contains the `Server` struct, which is the main entry point for the server.
/// The key store for the server.
pub mod key_store;
/// The registry cache for the server.
pub mod registry_cache;
/// The tick rate manager for the server.
pub mod tick_rate_manager;

use std::{sync::Arc, time::Instant};

use parking_lot::RwLock;
use steel_protocol::packets::game::{CLogin, CommonPlayerSpawnInfo};
use steel_registry::Registry;
use steel_utils::{Identifier, types::GameType};
use tick_rate_manager::TickRateManager;
use tokio::{runtime::Runtime, task::spawn_blocking};
use tokio_util::sync::CancellationToken;

use crate::{
    config::STEEL_CONFIG,
    player::Player,
    server::{key_store::KeyStore, registry_cache::RegistryCache},
    world::World,
};

/// The main server struct.
pub struct Server {
    /// The key store for the server.
    pub key_store: KeyStore,
    /// The registry for the server.
    pub registry: Arc<Registry>,
    /// The registry cache for the server.
    pub registry_cache: RegistryCache,
    /// A list of all the worlds on the server.
    pub worlds: Vec<Arc<World>>,
    /// The tick rate manager for the server.
    pub tick_rate_manager: RwLock<TickRateManager>,
}

impl Server {
    /// Creates a new server.
    pub async fn new(chunk_runtime: Arc<Runtime>) -> Self {
        let start = Instant::now();
        let mut registry = Registry::new_vanilla();
        registry.freeze();
        log::info!("Vanilla registry loaded in {:?}", start.elapsed());

        let registry = Arc::new(registry);
        let registry_cache = RegistryCache::new(&registry).await;

        Server {
            key_store: KeyStore::create(),
            worlds: vec![Arc::new(World::new(&registry.blocks, chunk_runtime))],
            registry,
            registry_cache,
            tick_rate_manager: RwLock::new(TickRateManager::new()),
        }
    }

    /// Adds a player to the server.
    pub fn add_player(&self, player: Arc<Player>) {
        player.connection.send_packet(CLogin {
            player_id: 0,
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
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 64,
            },
            enforces_secure_chat: true,
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
