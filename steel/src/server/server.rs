use std::sync::Arc;

use steel_protocol::packets::game::c_login_packet::{CLoginPacket, CommonPlayerSpawnInfo};
use steel_registry::Registry;
use steel_utils::ResourceLocation;
use steel_utils::types::GameType;
use steel_world::player::player::Player;
use steel_world::server::server::WorldServer;
use steel_world::world::world::World;
use tokio::time::Instant;

use crate::STEEL_CONFIG;
use crate::network::registry_cache::RegistryCache;
use crate::server::key_store::KeyStore;

pub struct Server {
    pub key_store: KeyStore,
    pub registry: Arc<Registry>,
    pub registry_cache: Arc<RegistryCache>,
    pub worlds: Vec<Arc<World>>,
}

impl Server {
    pub async fn new() -> Self {
        let start = Instant::now();
        let mut registry = Registry::new_vanilla();
        registry.freeze();
        log::info!("Vanilla registry loaded in {:?}", start.elapsed());

        let registry = Arc::new(registry);
        let registry_cache = Arc::new(RegistryCache::new(&registry).await);

        Server {
            key_store: KeyStore::new(),
            registry,
            worlds: vec![Arc::new(World::new())],
            registry_cache,
        }
    }
}

impl WorldServer for Server {
    fn add_player(&self, player: Player) {
        player.enqueue_packet(CLoginPacket::new(
            0,
            false,
            vec![ResourceLocation::vanilla_static("overworld")],
            5 as i32,
            STEEL_CONFIG.view_distance as i32,
            STEEL_CONFIG.simulation_distance as i32,
            false,
            false,
            false,
            CommonPlayerSpawnInfo::new(
                0,
                ResourceLocation::vanilla_static("overworld"),
                0,
                GameType::Survival,
                None,
                false,
                false,
                None,
                0,
                64,
            ),
            false,
        ));
        self.worlds[0].add_player(player);
    }
}
