//! This module contains the `JavaConnection` struct, which is used to represent a connection to a Java client.
use std::{
    io::Cursor,
    sync::{Arc, Weak, atomic::Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::player::Player;
use steel_protocol::{
    packet_reader::TCPNetworkDecoder,
    packet_traits::{ClientPacket, CompressionInfo, EncodedPacket, ServerPacket},
    packet_writer::TCPNetworkEncoder,
    packets::{
        common::{CDisconnect, CKeepAlive, SCustomPayload, SKeepAlive},
        game::{
            SChunkBatchReceived, SClientTickEnd, SMovePlayerPos, SMovePlayerPosRot, SMovePlayerRot,
            SPlayerLoad,
        },
    },
    utils::{ConnectionProtocol, EnqueuedPacket, PacketError, RawPacket},
};
use steel_registry::packets::play;
use steel_utils::{text::TextComponent, translations};
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

struct KeepAliveTracker {
    alive_time: u64,
    alive_pending: bool,
    alive_id: u64,
}

/// A connection to a Java client.
pub struct JavaConnection {
    outgoing_packets: UnboundedSender<EnqueuedPacket>,
    cancel_token: CancellationToken,
    compression: Option<CompressionInfo>,
    network_writer: Arc<Mutex<TCPNetworkEncoder<BufWriter<OwnedWriteHalf>>>>,
    id: u64,

    player: Weak<Player>,
    keep_alive_tracker: parking_lot::Mutex<KeepAliveTracker>,
    latency: parking_lot::Mutex<u32>,
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
            keep_alive_tracker: parking_lot::Mutex::new(KeepAliveTracker {
                alive_time: 0,
                alive_pending: false,
                alive_id: 0,
            }),
            latency: parking_lot::Mutex::new(0),
        }
    }

    /// Ticks the connection.
    pub fn tick(&self) {
        self.keep_connection_alive();
    }

    fn keep_connection_alive(&self) {
        let mut tracker = self.keep_alive_tracker.lock();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        if now - tracker.alive_time >= 15000 {
            if tracker.alive_pending {
                self.disconnect(translations::DISCONNECT_TIMEOUT.msg());
            } else {
                tracker.alive_pending = true;
                tracker.alive_id = now;
                tracker.alive_time = now;
                self.send_packet(CKeepAlive::new(tracker.alive_id as i64));
            }
        }
    }

    /// Handles a keep alive packet.
    #[allow(clippy::cast_possible_truncation)]
    fn handle_keep_alive(&self, packet: SKeepAlive) {
        let mut tracker = self.keep_alive_tracker.lock();
        if tracker.alive_pending && packet.id as u64 == tracker.alive_id {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            let time = now.saturating_sub(tracker.alive_time) as u32;
            tracker.alive_pending = false;
            drop(tracker);
            let mut latency = self.latency.lock();
            *latency = (*latency * 3 + time) / 4;
        } else {
            self.disconnect(translations::DISCONNECT_TIMEOUT.msg());
        }
    }

    /// Disconnects the client.
    pub fn disconnect(&self, reason: impl Into<TextComponent>) {
        self.send_packet(CDisconnect::new(reason.into()));
        self.close();
    }

    /// Sends a packet to the client.
    ///
    /// # Panics
    /// - If the packet fails to be written to the buffer.
    /// - If the packet fails to be sent through the channel.
    pub fn send_packet<P: ClientPacket>(&self, packet: P) {
        let packet = EncodedPacket::write_vec(packet, ConnectionProtocol::Play)
            .expect("Failed to write packet");
        self.outgoing_packets
            .send(EnqueuedPacket::RawData(packet))
            .expect("Failed to send packet");
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
                let _ = SClientTickEnd::read_packet(data)?;
                player.handle_client_tick_end();
            }
            play::S_CHUNK_BATCH_RECEIVED => {
                let packet = SChunkBatchReceived::read_packet(data)?;
                player
                    .chunk_sender
                    .lock()
                    .on_chunk_batch_received_by_client(packet.desired_chunks_per_tick);
            }
            play::S_KEEP_ALIVE => {
                self.handle_keep_alive(SKeepAlive::read_packet(data)?);
            }
            play::S_MOVE_PLAYER_POS => {
                player.handle_move_player(SMovePlayerPos::read_packet(data)?.into());
            }
            play::S_MOVE_PLAYER_POS_ROT => {
                player.handle_move_player(SMovePlayerPosRot::read_packet(data)?.into());
            }
            play::S_MOVE_PLAYER_ROT => {
                player.handle_move_player(SMovePlayerRot::read_packet(data)?.into());
            }
            play::S_PLAYER_LOADED => {
                let _ = SPlayerLoad::read_packet(data)?;
                player.client_loaded.store(true, Ordering::Relaxed);
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
                            log::debug!("Failed to get raw packet from client {}: {err}", self.id);
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
                        //log::warn!(
                        //    "Internal packet_sender_recv channel closed for client {}",
                        //    self.id
                        //);
                        self.close();
                    }
                }
            }
        }

        let player = self.player.upgrade().expect("Player is not available");
        let world = player.world.clone();
        world.remove_player(player).await;
    }
}
