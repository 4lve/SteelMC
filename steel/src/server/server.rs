use std::sync::Arc;

use steel_registry::Registry;
use steel_world::player::player::Player;
use steel_world::server::server::WorldServer;
use steel_world::world::world::World;
use tokio::time::Instant;

use crate::server::key_store::KeyStore;

pub struct Server {
    pub key_store: KeyStore,
    pub registry: Arc<Registry>,
    pub worlds: Vec<Arc<World>>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn new() -> Self {
        let start = Instant::now();
        let mut registry = Registry::new_vanilla();
        registry.freeze();
        log::info!("Vanilla registry loaded in {:?}", start.elapsed());

        Server {
            key_store: KeyStore::new(),
            registry: Arc::new(registry),
            worlds: vec![Arc::new(World::new())],
        }
    }
}

impl WorldServer for Server {
    fn add_player(&self, player: Player) {
        self.worlds[0].add_player(player);
    }
}
