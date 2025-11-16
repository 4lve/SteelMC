//! # Steel
//!
//! The main library for the Steel Minecraft server.
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::cargo,
    missing_docs,
    clippy::unwrap_used
)]
#![allow(
    clippy::single_call_fn,
    clippy::multiple_inherent_impl,
    clippy::shadow_unrelated,
    clippy::missing_errors_doc,
    clippy::struct_excessive_bools,
    clippy::needless_pass_by_value,
    clippy::cargo_common_metadata
)]
use crate::network::JavaTcpClient;
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};
use steel_core::{config::STEEL_CONFIG, server::Server};
use tokio::{net::TcpListener, select};
use tokio_util::sync::CancellationToken;

/// The networking module.
pub mod network;

/// The supported Minecraft version.
pub const MC_VERSION: &str = "1.21.10";

/// The main server struct.
pub struct SteelServer {
    /// The TCP listener for incoming connections.
    pub tcp_listener: TcpListener,
    /// The cancellation token for graceful shutdown.
    pub cancel_token: CancellationToken,
    /// The next client ID to be assigned.
    pub client_id: u64,
    /// The shared server state.
    pub server: Arc<Server>,
}

impl SteelServer {
    /// Creates a new Steel server.
    ///
    /// # Panics
    /// This function will panic if the TCP listener fails to bind to the server address.
    pub async fn new() -> Self {
        log::info!("Starting Steel Server");

        let server = Server::new().await;

        Self {
            tcp_listener: TcpListener::bind(SocketAddrV4::new(
                Ipv4Addr::UNSPECIFIED,
                STEEL_CONFIG.server_port,
            ))
            .await
            .expect("Failed to bind to server address"),
            cancel_token: CancellationToken::new(),
            client_id: 0,
            server: Arc::new(server),
        }
    }

    /// Starts the server and begins accepting connections.
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
                    // Java_client won't drop until the incoming and outcoming task close
                    // So we dont need to care about them here anymore
                }
            }
        }
    }
}
