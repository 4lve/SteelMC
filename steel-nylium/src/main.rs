use std::sync::Arc;
use std::sync::LazyLock;

use async_trait::async_trait;
use nylium::{Nylium, NyliumLogger};
use nylium_adapter::{Global, NyliumConfig, NyliumServer, Player};
use steel::SteelServer;
use tokio::sync::Mutex;

static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to tokio runtime")
});

fn main() {
    let logger = NyliumLogger::init();

    RUNTIME.block_on(async {
        let server = SteelServerNylium::new().await;
        Nylium::new(server, logger).run();
    });
}

#[derive(Clone)]
struct SteelServerNylium(Arc<Mutex<SteelServer>>);

impl SteelServerNylium {
    async fn new() -> Self {
        let server = SteelServer::new().await;
        Self(Arc::new(Mutex::new(server)))
    }
}

impl Global for SteelServerNylium {}

#[async_trait]
impl NyliumServer<DummyConfig> for SteelServerNylium {
    async fn start(&self) {
        let server = self.0.clone();
        RUNTIME.spawn(async move {
            let mut server = server.lock().await;
            server.start().await;
        });
    }

    async fn stop(&self) {
        let server = self.0.lock().await;
        server.stop();
    }

    fn get_config(&self) -> DummyConfig {
        DummyConfig {
            server_port: 25565,
            seed: String::new(),
            max_players: 20,
            view_distance: 10,
            simulation_distance: 10,
            online_mode: true,
            encryption: true,
            motd: "A Minecraft Server".to_string(),
            use_favicon: false,
            favicon: String::new(),
            enforce_secure_chat: true,
        }
    }

    fn update_config(&self, _config: &DummyConfig) -> bool {
        println!("Config updated");
        true
    }

    async fn send_command(&self, command: &str) {
        println!("Command received: {}", command);
    }

    async fn get_players(&self) -> Vec<Player> {
        let mut players = Vec::new();

        self.0.lock().await.server.worlds[0]
            .players
            .iter_async(|_k, p| {
                players.push(Player {
                    id: p.gameprofile.id.clone(),
                    name: p.gameprofile.name.clone(),
                });
                false
            })
            .await;

        players
    }
}

#[derive(NyliumConfig)]
pub struct DummyConfig {
    /// The port the server will listen on.
    pub server_port: u16,
    /// The seed for the world generator.
    pub seed: String,
    /// The maximum number of players that can be on the server at once.
    pub max_players: u32,
    /// The view distance of the server.
    pub view_distance: u8,
    /// The simulation distance of the server.
    pub simulation_distance: u8,
    /// Whether the server is in online mode.
    pub online_mode: bool,
    /// Whether the server should use encryption.
    pub encryption: bool,
    /// The message of the day.
    pub motd: String,
    /// Whether to use a favicon.
    pub use_favicon: bool,
    /// The path to the favicon.
    pub favicon: String,
    /// Whether to enforce secure chat.
    pub enforce_secure_chat: bool,
}
