#![warn(clippy::all, clippy::pedantic, clippy::cargo)]
#![allow(
    clippy::single_call_fn,
    clippy::multiple_inherent_impl,
    clippy::shadow_unrelated,
    clippy::missing_errors_doc,
    clippy::struct_excessive_bools,
    clippy::needless_pass_by_value,
    clippy::cargo_common_metadata
)]
use crate::{network::JavaTcpClient, server::Server, steel_config::SteelConfig};
use std::{
    path::Path,
    sync::{Arc, LazyLock},
};
use tokio::{net::TcpListener, select};
use tokio_util::sync::CancellationToken;

pub mod network;
pub mod server;
pub mod steel_config;

pub const MC_VERSION: &str = "1.21.10";

pub static STEEL_CONFIG: LazyLock<SteelConfig> =
    LazyLock::new(|| SteelConfig::load_or_create(Path::new("config/steel_config.json5")));

pub struct SteelServer {
    pub tcp_listener: TcpListener,
    pub cancel_token: CancellationToken,
    pub client_id: u64,
    pub server: Arc<Server>,
}

impl SteelServer {
    /// # Panics
    /// This function will panic if the TCP listener fails to bind to the server address.
    pub async fn new() -> Self {
        log::info!("Starting Steel Server");

        let server = Server::new().await;

        Self {
            tcp_listener: TcpListener::bind(STEEL_CONFIG.server_address)
                .await
                .unwrap(),
            cancel_token: CancellationToken::new(),
            client_id: 0,
            server: Arc::new(server),
        }
    }

    pub async fn start(&mut self) {
        log::info!("Started Steel Server");

        loop {
            select! {
                () = self.cancel_token.cancelled() => {
                    break;
                }
                accept_result = self.tcp_listener.accept() => {
                    let Ok((connection, address)) = accept_result else {
                        continue;
                    };
                    if let Err(e) = connection.set_nodelay(true) {
                        log::warn!("Failed to set TCP_NODELAY: {e}");
                    }
                    let (java_client, sender_recv, net_reader) = JavaTcpClient::new(connection, address, self.client_id, self.cancel_token.child_token(), self.server.clone());
                    self.client_id = self.client_id.wrapping_add(1);
                    log::info!("Accepted connection from Java Edition: {address} (id {})", self.client_id);

                    let java_client = Arc::new(java_client);
                    java_client.start_outgoing_packet_task(sender_recv);
                    java_client.start_incoming_packet_task(net_reader);
                    // Java_client won't drop untill the incoming and outcoming task close
                    // So we dont need to care about them here anymore
                }
            }
        }
    }
}
