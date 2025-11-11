mod key_store;
mod registry_cache;
use std::{sync::Arc, time::Instant};

use steel_protocol::packets::game::{CLogin, CommonPlayerSpawnInfo};
use steel_registry::Registry;
use steel_utils::{Identifier, types::GameType};

use crate::{
    config::STEEL_CONFIG,
    player::Player,
    server::{key_store::KeyStore, registry_cache::RegistryCache},
    world::World,
};

pub struct Server {
    pub key_store: KeyStore,
    pub registry: Arc<Registry>,
    pub registry_cache: RegistryCache,
    pub worlds: Vec<Arc<World>>,
}

impl Server {
    pub async fn new() -> Self {
        let start = Instant::now();
        let mut registry = Registry::new_vanilla();
        registry.freeze();
        log::info!("Vanilla registry loaded in {:?}", start.elapsed());

        let registry = Arc::new(registry);
        let registry_cache = RegistryCache::new(&registry).await;

        Server {
            key_store: KeyStore::create(),
            registry,
            worlds: vec![Arc::new(World::new())],
            registry_cache,
        }
    }

    pub fn run(self: Arc<Self>) {}

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
}
