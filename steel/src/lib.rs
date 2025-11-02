use crate::{
    network::java_tcp_client::JavaTcpClient, server::server::Server, steel_config::SteelConfig,
};
use std::{
    path::Path,
    sync::{Arc, LazyLock},
};
use tokio::{net::TcpListener, select};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

pub mod network;
pub mod server;
pub mod steel_config;

pub const MC_VERSION: &str = "1.21.10";

pub static STEEL_CONFIG: LazyLock<SteelConfig> =
    LazyLock::new(|| SteelConfig::load_or_create(Path::new("config/steel_config.json5")));

pub struct SteelServer {
    pub tcp_listener: TcpListener,
    pub cancellation_token: CancellationToken,
    pub client_id: u64,
    pub server: Arc<Server>,
}

impl SteelServer {
    pub async fn new() -> Self {
        log::info!("Starting Steel Server");

        let server = Server::new().await;

        Self {
            tcp_listener: TcpListener::bind(STEEL_CONFIG.server_address)
                .await
                .unwrap(),
            cancellation_token: CancellationToken::new(),
            client_id: 0,
            server: Arc::new(server),
        }
    }

    pub async fn start(&mut self) {
        log::info!("Started Steel Server");

        let tasks = TaskTracker::new();

        loop {
            let cancellation_token_clone = self.cancellation_token.clone();

            select! {
                _ = cancellation_token_clone.cancelled() => {
                    break;
                }
                accept_result = self.tcp_listener.accept() => {
                    let (connection, address) = accept_result.unwrap();
                    if let Err(e) = connection.set_nodelay(true) {
                        log::warn!("Failed to set TCP_NODELAY: {e}");
                    }
                    let mut java_client = JavaTcpClient::new(connection, address, self.client_id, cancellation_token_clone.child_token(), self.server.clone());
                    self.client_id += 1;
                    log::info!("Accepted connection from Java Edition: {address} (id {})", self.client_id);
                    java_client.start_incoming_packet_task();
                    java_client.start_outgoing_packet_task();
                    let java_client = Arc::new(java_client);
                    tasks.spawn(async move {
                        java_client.process_packets().await;
                        java_client.close();
                        java_client.await_tasks().await;
                    });
                }
            }
        }
    }
}
