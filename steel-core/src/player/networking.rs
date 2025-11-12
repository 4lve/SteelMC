//! This module contains the `JavaConnection` struct, which is used to represent a connection to a Java client.
use std::{
    io::Cursor,
    sync::{Arc, Weak},
};

use crate::player::{Player, chunk_sender::ChunkSender};
use steel_protocol::{
    packet_reader::TCPNetworkDecoder,
    packet_traits::{ClientPacket, CompressionInfo, EncodedPacket, ServerPacket},
    packet_writer::TCPNetworkEncoder,
    packets::common::SCustomPayload,
    utils::{ConnectionProtocol, EnqueuedPacket, PacketError, RawPacket},
};
use steel_registry::packets::play;
use tokio::{
    io::{BufReader, BufWriter},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    select,
    sync::{
        Mutex,
        mpsc::{UnboundedReceiver, UnboundedSender},
    },
};
use tokio_util::sync::CancellationToken;

/// A connection to a Java client.
pub struct JavaConnection {
    outgoing_packets: UnboundedSender<EnqueuedPacket>,
    cancel_token: CancellationToken,
    compression: Option<CompressionInfo>,
    network_writer: Arc<Mutex<TCPNetworkEncoder<BufWriter<OwnedWriteHalf>>>>,
    id: u64,

    player: Weak<Player>,
    #[allow(unused)]
    chunk_sender: ChunkSender,
}

impl JavaConnection {
    /// Creates a new `JavaConnection`.
    pub fn new(
        outgoing_packets: UnboundedSender<EnqueuedPacket>,
        cancel_token: CancellationToken,
        compression: Option<CompressionInfo>,
        network_writer: Arc<Mutex<TCPNetworkEncoder<BufWriter<OwnedWriteHalf>>>>,
        id: u64,
        player: Weak<Player>,
    ) -> Self {
        Self {
            outgoing_packets,
            cancel_token,
            compression,
            network_writer,
            id,
            player,
            chunk_sender: ChunkSender::default(),
        }
    }

    /// Sends a packet to the client.
    ///
    /// # Panics
    /// - If the packet fails to be written to the buffer.
    /// - If the packet fails to be sent through the channel.
    pub fn send_packet<P: ClientPacket>(&self, packet: P) {
        let packet = EncodedPacket::write_vec(packet, ConnectionProtocol::Play).unwrap();
        self.outgoing_packets
            .send(EnqueuedPacket::RawData(packet))
            .unwrap();
    }

    /// Closes the connection.
    pub fn close(&self) {
        self.cancel_token.cancel();
    }

    /// Returns whether the connection is closed.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// Waits for the connection to be closed.
    pub async fn wait_for_close(&self) {
        self.cancel_token.cancelled().await;
    }

    /// Processes a packet from the client.
    pub fn process_packet(
        self: &Arc<Self>,
        packet: RawPacket,
        player: Arc<Player>,
    ) -> Result<(), PacketError> {
        let data = &mut Cursor::new(packet.payload);

        match packet.id {
            play::C_CUSTOM_PAYLOAD => {
                player.handle_custom_payload(SCustomPayload::read_packet(data)?);
            }
            play::S_CLIENT_TICK_END => {
                player.handle_client_tick_end();
            }
            id => log::info!("play packet id {id} is not known"),
        }
        Ok(())
    }

    /// Listens for packets from the client.
    pub async fn listener(
        self: Arc<Self>,
        mut reader: TCPNetworkDecoder<BufReader<OwnedReadHalf>>,
    ) {
        loop {
            select! {
                () = self.wait_for_close() => {
                    break;
                }
                packet = reader.get_raw_packet() => {
                    match packet {
                        Ok(packet) => {
                            if let Some(player) = self.player.upgrade()
                                && let Err(err) = self.process_packet(packet, player) {
                                    log::warn!(
                                        "Failed to get packet from client {}: {err}",
                                        self.id
                                    );
                                }
                        }
                        Err(err) => {
                            log::info!("Failed to get raw packet from client {}: {err}", self.id);
                            self.close();
                        }
                    }
                }
            }
        }
    }

    /// Sends packets to the client.
    ///
    /// # Panics
    /// - If the player is not available.
    pub async fn sender(self: Arc<Self>, mut sender_recv: UnboundedReceiver<EnqueuedPacket>) {
        loop {
            select! {
                () = self.wait_for_close() => {
                    break;
                }
                packet = sender_recv.recv() => {
                    if let Some(packet) = packet {

                        let Some(encoded_packet) = (match packet {
                            EnqueuedPacket::EncodedPacket(packet) => Some(packet),
                            EnqueuedPacket::RawData(packet) => {
                                EncodedPacket::from_data(packet, self.compression)
                                    .await
                                    .ok()
                            }
                        }) else {
                            log::warn!("Failed to convert packet to encoded packet for client {}", self.id);
                            continue;
                        };

                        if let Err(err) = self.network_writer.lock().await.write_packet(&encoded_packet).await
                        {
                            log::warn!("Failed to send packet to client {}: {err}", self.id);
                            self.close();
                        }

                    } else {
                        log::warn!(
                            "Internal packet_sender_recv channel closed for client {}",
                            self.id
                        );
                        self.close();
                    }
                }
            }
        }

        let player = self.player.upgrade().unwrap();
        let world = player.world.clone();
        world.remove_player(player);
    }
}
