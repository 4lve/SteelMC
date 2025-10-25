use std::{
    net::SocketAddr,
    sync::{Arc, atomic::AtomicBool},
};

use crossbeam::atomic::AtomicCell;
use steel_protocol::{
    packet_reader::TCPNetworkDecoder,
    packet_traits::{CompressionInfo, EncodedPacket},
    packet_writer::TCPNetworkEncoder,
    packets::{
        clientbound::{CBoundConfiguration, CBoundLogin, CBoundPacket, CBoundPlay},
        common::c_disconnect_packet::CDisconnectPacket,
        handshake::ClientIntent,
        login::c_login_disconnect_packet::CLoginDisconnectPacket,
        serverbound::{
            SBoundConfiguration, SBoundHandshake, SBoundLogin, SBoundPacket, SBoundPlay,
            SBoundStatus,
        },
    },
    utils::{ConnectionProtocol, PacketError, RawPacket},
};
use steel_utils::text::TextComponent;
use thiserror::Error;
use tokio::{
    io::{BufReader, BufWriter},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::{
        Mutex,
        broadcast::{self, Receiver, Sender},
    },
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use crate::{
    network::{
        game_profile::GameProfile,
        login::{handle_hello, handle_key},
        status::{handle_ping_request, handle_status_request},
    },
    server::server::Server,
};

#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("failed to decrypt shared secret")]
    FailedDecrypt,
    #[error("shared secret has the wrong length")]
    SharedWrongLength,
}

#[derive(Clone)]
pub enum EnqueuedPacket {
    Packet(CBoundPacket),
    EncodedPacket(EncodedPacket),
}

pub struct JavaTcpClient {
    pub id: u64,
    /// The client's game profile information.
    pub gameprofile: Mutex<Option<GameProfile>>,
    /// The current connection state of the client (e.g., Handshaking, Status, Play).
    pub connection_protocol: Arc<AtomicCell<ConnectionProtocol>>,
    /// The client's IP address.
    pub address: Mutex<SocketAddr>,
    /// A collection of tasks associated with this client. The tasks await completion when removing the client.
    tasks: TaskTracker,
    /// A token to cancel the client's operations. Called when the connection is closed. Or client is removed.
    cancel_token: CancellationToken,

    packet_receiver: Mutex<Option<Receiver<Arc<SBoundPacket>>>>,
    pub packet_recv_sender: Arc<Sender<Arc<SBoundPacket>>>,

    /// A queue of serialized packets to send to the network
    outgoing_queue: Sender<EnqueuedPacket>,
    /// A queue of serialized packets to send to the network
    outgoing_queue_recv: Option<Receiver<EnqueuedPacket>>,
    /// The packet encoder for outgoing packets.
    network_writer: Arc<Mutex<TCPNetworkEncoder<BufWriter<OwnedWriteHalf>>>>,
    /// The packet decoder for incoming packets.
    network_reader: Arc<Mutex<TCPNetworkDecoder<BufReader<OwnedReadHalf>>>>,
    compression_info: Arc<AtomicCell<Option<CompressionInfo>>>,
    pub(crate) has_requested_status: AtomicBool,
    pub server: Arc<Server>,
    pub challenge: AtomicCell<Option<[u8; 4]>>,
}

impl JavaTcpClient {
    pub fn new(
        tcp_stream: TcpStream,
        address: SocketAddr,
        id: u64,
        cancel_token: CancellationToken,
        server: Arc<Server>,
    ) -> Self {
        let (read, write) = tcp_stream.into_split();
        let (send, recv) = broadcast::channel(128);

        let (packet_recv_sender, packet_receiver) = broadcast::channel(128);

        Self {
            id,
            gameprofile: Mutex::new(None),
            address: Mutex::new(address),
            connection_protocol: Arc::new(AtomicCell::new(ConnectionProtocol::HANDSHAKING)),
            tasks: TaskTracker::new(),
            cancel_token,

            packet_receiver: Mutex::new(Some(packet_receiver)),
            packet_recv_sender: Arc::new(packet_recv_sender),
            outgoing_queue: send,
            outgoing_queue_recv: Some(recv),
            network_writer: Arc::new(Mutex::new(TCPNetworkEncoder::new(BufWriter::new(write)))),
            network_reader: Arc::new(Mutex::new(TCPNetworkDecoder::new(BufReader::new(read)))),
            has_requested_status: AtomicBool::new(false),
            compression_info: Arc::new(AtomicCell::new(None)),
            server: server,
            challenge: AtomicCell::new(None),
        }
    }

    async fn set_encryption(
        &self,
        shared_secret: &[u8], // decrypted
    ) -> Result<(), EncryptionError> {
        let crypt_key: [u8; 16] = shared_secret
            .try_into()
            .map_err(|_| EncryptionError::SharedWrongLength)?;
        self.network_reader.lock().await.set_encryption(&crypt_key);
        self.network_writer.lock().await.set_encryption(&crypt_key);
        Ok(())
    }

    pub async fn set_compression(&self, compression: CompressionInfo) {
        if compression.level > 9 {
            log::error!("Invalid compression level! Clients will not be able to read this!");
        }

        self.network_reader
            .lock()
            .await
            .set_compression(compression.threshold as usize);

        self.compression_info.store(Some(compression));
    }

    async fn get_packet(&self) -> Option<RawPacket> {
        let mut network_reader = self.network_reader.lock().await;
        tokio::select! {
            () = self.cancel_token.cancelled() => {
                log::debug!("Canceling player packet processing");
                None
            },
            packet_result = network_reader.get_raw_packet() => {
                match packet_result {
                    Ok(packet) => Some(packet),
                    Err(err) => {
                        if !matches!(err, PacketError::ConnectionClosed) {
                            log::warn!("Failed to decode packet from client {}: {}", self.id, err);
                            let text = format!("Error while reading incoming packet {err}");
                            self.kick(TextComponent::text(text)).await;
                        }
                        None
                    }
                }
            }
        }
    }

    pub fn close(&self) {
        self.cancel_token.cancel();
    }

    pub async fn await_tasks(&self) {
        self.tasks.close();
        self.tasks.wait().await;
    }

    pub async fn send_packet_now(&self, packet: CBoundPacket) {
        let compression_info = self.compression_info.load();
        let encoded_packet = EncodedPacket::from_packet(&packet, compression_info)
            .await
            .unwrap();
        if let Err(err) = self
            .network_writer
            .lock()
            .await
            .write_encoded_packet(&encoded_packet)
            .await
        {
            // It is expected that the packet will fail if we are cancelled
            if !self.cancel_token.is_cancelled() {
                log::warn!("Failed to send packet to client {}: {}", self.id, err);
                // We now need to close the connection to the client since the stream is in an
                // unknown state
                self.close();
            }
        }
    }

    pub fn enqueue_packet(&self, packet: CBoundPacket) -> Result<(), PacketError> {
        self.outgoing_queue
            .send(EnqueuedPacket::Packet(packet))
            .map_err(|e| {
                PacketError::SendError(format!(
                    "Failed to send packet to client {}: {}",
                    self.id, e
                ))
            })?;
        Ok(())
    }

    pub fn enqueue_encoded_packet(&self, packet: EncodedPacket) -> Result<(), PacketError> {
        self.outgoing_queue
            .send(EnqueuedPacket::EncodedPacket(packet))
            .map_err(|e| {
                PacketError::SendError(format!(
                    "Failed to send packet to client {}: {}",
                    self.id, e
                ))
            })?;
        Ok(())
    }

    pub async fn encode_packet(
        packet: CBoundPacket,
        compression_info: Option<CompressionInfo>,
    ) -> Result<EncodedPacket, PacketError> {
        let encoded_packet = EncodedPacket::from_packet(&packet, compression_info)
            .await
            .map_err(|e| {
                PacketError::SendError(format!("Failed to create encoded packet: {}", e))
            })?;
        Ok(encoded_packet)
    }

    /// Starts a task that will send packets to the client from the outgoing packet queue.
    /// This task will run until the client is closed or the cancellation token is cancelled.
    pub fn start_outgoing_packet_task(&mut self) {
        let mut sender_recv = self
            .outgoing_queue_recv
            .take()
            .expect("This was set in the new fn");
        let cancel_token = self.cancel_token.clone();
        let writer = self.network_writer.clone();
        let id = self.id;
        let compression_info = self.compression_info.clone();

        self.tasks.spawn(async move {
            let cancel_token_clone = cancel_token.clone();

            cancel_token
                .run_until_cancelled(async move {
                    loop {
                        match sender_recv.recv().await {
                            Ok(packet) => {
                                let encoded_packet = match packet {
                                    EnqueuedPacket::EncodedPacket(packet) => packet,
                                    EnqueuedPacket::Packet(packet) => {
                                        Self::encode_packet(packet, compression_info.load())
                                            .await
                                            .unwrap()
                                    }
                                };

                                if let Err(err) = writer
                                    .lock()
                                    .await
                                    .write_encoded_packet(&encoded_packet)
                                    .await
                                {
                                    log::warn!("Failed to send packet to client {}: {}", id, err);
                                    cancel_token_clone.cancel();
                                }
                            }
                            Err(err) => {
                                log::warn!(
                                    "Internal packet_sender_recv channel closed for client {}: {}",
                                    id,
                                    err
                                );
                                cancel_token_clone.cancel();
                            }
                        }
                    }
                })
                .await;
        });
    }
}

impl JavaTcpClient {
    pub fn start_incoming_packet_task(&mut self) {
        let network_reader = self.network_reader.clone();
        let cancel_token = self.cancel_token.clone();
        let id = self.id;
        let packet_recv_sender = self.packet_recv_sender.clone();
        let connection_protocol = self.connection_protocol.clone();

        self.tasks.spawn(async move {
            let cancel_token_clone = cancel_token.clone();
            cancel_token
                .run_until_cancelled(async move {
                    loop {
                        let mut network_reader = network_reader.lock().await;

                        let packet = network_reader.get_raw_packet().await;
                        match packet {
                            Ok(packet) => {
                                log::info!("Received packet: {:?}", packet.id);
                                match SBoundPacket::from_raw_packet(
                                    packet,
                                    connection_protocol.load(),
                                ) {
                                    Ok(packet) => {
                                        packet_recv_sender.send(Arc::new(packet)).unwrap();
                                    }
                                    Err(err) => {
                                        log::warn!(
                                            "Failed to get packet from client {}: {}",
                                            id,
                                            err
                                        );
                                        cancel_token_clone.cancel();
                                    }
                                }
                            }
                            Err(err) => {
                                if cancel_token_clone.is_cancelled() {
                                    break;
                                }
                                log::info!("Failed to get raw packet from client {}: {}", id, err);
                                cancel_token_clone.cancel();
                            }
                        }
                    }
                })
                .await;
        });
    }

    // This code is used but the linter doesn't notice it
    #[allow(dead_code)]
    pub async fn process_packets(self: &Arc<Self>) {
        let mut packet_receiver = self
            .packet_receiver
            .lock()
            .await
            .take()
            .expect("This was set in the new fn or the function was called twice");

        self.cancel_token
            .run_until_cancelled(async move {
                loop {
                    let packet = packet_receiver.recv().await.unwrap();
                    match &*packet {
                        SBoundPacket::Handshake(packet) => self.handle_handshake(packet).await,
                        SBoundPacket::Status(packet) => self.handle_status(packet).await,
                        SBoundPacket::Login(packet) => self.handle_login(packet).await,
                        SBoundPacket::Configuration(packet) => {
                            self.handle_configuration(packet).await
                        }
                        SBoundPacket::Play(packet) => self.handle_play(packet).await,
                    }
                }
            })
            .await;
    }

    fn assert_protocol(&self, protocol: ConnectionProtocol) -> bool {
        if self.connection_protocol.load() != protocol {
            self.close();
            return false;
        }
        true
    }

    pub async fn handle_handshake(&self, packet: &SBoundHandshake) {
        if !self.assert_protocol(ConnectionProtocol::HANDSHAKING) {
            return;
        }
        match packet {
            SBoundHandshake::Intention(packet) => {
                let intent = match packet.intention {
                    ClientIntent::LOGIN => ConnectionProtocol::LOGIN,
                    ClientIntent::STATUS => ConnectionProtocol::STATUS,
                    ClientIntent::TRANSFER => ConnectionProtocol::LOGIN,
                };
                self.connection_protocol.store(intent);

                if intent != ConnectionProtocol::STATUS {
                    //TODO: Handle client version being too low or high
                }
            }
        }
    }

    pub async fn handle_status(&self, packet: &SBoundStatus) {
        if !self.assert_protocol(ConnectionProtocol::STATUS) {
            return;
        }

        match packet {
            SBoundStatus::StatusRequest(packet) => handle_status_request(self, packet).await,
            SBoundStatus::PingRequest(packet) => handle_ping_request(self, packet).await,
        }
    }

    pub async fn handle_login(&self, packet: &SBoundLogin) {
        if !self.assert_protocol(ConnectionProtocol::LOGIN) {
            return;
        }

        match packet {
            SBoundLogin::Hello(packet) => handle_hello(self, packet).await,
            SBoundLogin::Key(packet) => handle_key(self, packet).await,
        }
    }

    pub async fn handle_configuration(&self, _packet: &SBoundConfiguration) {
        if !self.assert_protocol(ConnectionProtocol::CONFIGURATION) {}
    }

    pub async fn handle_play(&self, _packet: &SBoundPlay) {
        if !self.assert_protocol(ConnectionProtocol::PLAY) {}
    }
}

impl JavaTcpClient {
    pub async fn kick(&self, reason: TextComponent) {
        match self.connection_protocol.load() {
            ConnectionProtocol::LOGIN => {
                let packet = CLoginDisconnectPacket::new(reason.0);
                self.send_packet_now(CBoundPacket::Login(CBoundLogin::LoginDisconnectPacket(
                    packet,
                )))
                .await;
            }
            ConnectionProtocol::CONFIGURATION => {
                let packet = CDisconnectPacket::new(reason.0);
                self.send_packet_now(CBoundPacket::Configuration(
                    CBoundConfiguration::Disconnect(packet),
                ))
                .await;
            }
            ConnectionProtocol::PLAY => {
                let packet = CDisconnectPacket::new(reason.0);
                self.send_packet_now(CBoundPacket::Play(CBoundPlay::Disconnect(packet)))
                    .await;
            }
            _ => {}
        }
        log::debug!("Closing connection for {}", self.id);
        self.close();
    }
}
