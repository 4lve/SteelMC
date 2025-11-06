use std::{
    io::Cursor,
    net::SocketAddr,
    sync::{Arc, Weak},
};

use crossbeam::atomic::AtomicCell;
use steel_protocol::{
    packet_reader::TCPNetworkDecoder,
    packet_traits::{ClientPacket, CompressionInfo, EncodedPacket, ServerPacket},
    packet_writer::TCPNetworkEncoder,
    packets::{
        common::{CDisconnect, SClientInformation, SCustomPayload},
        config::{SFinishConfiguration, SSelectKnownPacks},
        game::SClientTickEnd,
        handshake::{ClientIntent, SClientIntention},
        login::{CLoginDisconnect, SHello, SKey, SLoginAcknowledged},
        status::{SPingRequest, SStatusRequest},
    },
    utils::{ConnectionProtocol, EnqueuedPacket, PacketError, RawPacket},
};
use steel_registry::packets::{config, handshake, login, play, status};
use steel_utils::text::TextComponent;
use steel_world::player::{GameProfile, Player};
use tokio::{
    io::{BufReader, BufWriter},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    select,
    sync::{
        Mutex, Notify, OnceCell,
        broadcast::{self, Sender, error::RecvError},
        mpsc::{self, UnboundedReceiver, UnboundedSender},
    },
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use crate::{
    network::status::{handle_ping_request, handle_status_request},
    server::Server,
};

#[derive(Clone, Debug)]
pub enum ConnectionUpdate {
    EnableEncryption([u8; 16]),
    EnableCompression(CompressionInfo),
}

pub struct JavaTcpClient {
    pub id: u64,
    /// The client's game profile information.
    pub gameprofile: Mutex<Option<GameProfile>>,
    /// The current connection state of the client (e.g., Handshaking, Status, Play).
    pub protocol: Arc<AtomicCell<ConnectionProtocol>>,
    /// The client's IP address.
    pub address: SocketAddr,
    /// A collection of tasks associated with this client. The tasks await completion when removing the client.
    tasks: TaskTracker,
    /// A token to cancel the client's operations. Called when the connection is closed. Or client is removed.
    pub cancel_token: CancellationToken,

    /// A queue of serialized packets to send to the network
    pub outgoing_queue: UnboundedSender<EnqueuedPacket>,
    /// The packet encoder for outgoing packets.
    network_writer: Arc<Mutex<TCPNetworkEncoder<BufWriter<OwnedWriteHalf>>>>,
    pub(crate) compression_info: Arc<AtomicCell<Option<CompressionInfo>>>,

    pub server: Arc<Server>,
    pub challenge: AtomicCell<Option<[u8; 4]>>,

    pub(crate) connection_updates: Sender<ConnectionUpdate>,
    pub(crate) connection_updated: Arc<Notify>,
    pub player: OnceCell<Weak<Player>>,
}

impl JavaTcpClient {
    #[must_use]
    pub fn new(
        tcp_stream: TcpStream,
        address: SocketAddr,
        id: u64,
        cancel_token: CancellationToken,
        server: Arc<Server>,
    ) -> (
        Self,
        UnboundedReceiver<EnqueuedPacket>,
        TCPNetworkDecoder<BufReader<OwnedReadHalf>>,
    ) {
        let (read, write) = tcp_stream.into_split();
        let (outgoing_queue, recv) = mpsc::unbounded_channel();
        let (connection_updates, _) = broadcast::channel(128);

        let client = Self {
            id,
            gameprofile: Mutex::new(None),
            address,
            protocol: Arc::new(AtomicCell::new(ConnectionProtocol::Handshake)),
            tasks: TaskTracker::new(),
            cancel_token,

            outgoing_queue,
            network_writer: Arc::new(Mutex::new(TCPNetworkEncoder::new(BufWriter::new(write)))),
            compression_info: Arc::new(AtomicCell::new(None)),
            server,
            challenge: AtomicCell::new(None),
            connection_updates,
            connection_updated: Arc::new(Notify::new()),
            player: OnceCell::new(),
        };

        (client, recv, TCPNetworkDecoder::new(BufReader::new(read)))
    }

    pub fn close(&self) {
        self.cancel_token.cancel();
    }

    /// # Panics
    /// This function will panic if the packet cannot be encoded. Should never happen.
    pub async fn send_bare_packet_now<P: ClientPacket>(&self, packet: P) {
        let compression_info = self.compression_info.load();
        let protocol = self.protocol.load();
        let packet = EncodedPacket::from_bare(packet, compression_info, protocol)
            .await
            .unwrap();

        if let Err(err) = self.network_writer.lock().await.write_packet(&packet).await
            && !self.cancel_token.is_cancelled()
        {
            log::warn!("Failed to send packet to client {}: {}", self.id, err);
            self.close();
        }
    }

    pub async fn send_packet_now(&self, packet: &EncodedPacket) {
        if let Err(err) = self.network_writer.lock().await.write_packet(packet).await
            && !self.cancel_token.is_cancelled()
        {
            log::warn!("Failed to send packet to client {}: {}", self.id, err);
            self.close();
        }
    }

    pub fn send_bare_packet<P: ClientPacket>(&self, packet: P) -> Result<(), PacketError> {
        let protocol = self.protocol.load();
        let buf = EncodedPacket::write_vec(packet, protocol)?;
        self.outgoing_queue
            .send(EnqueuedPacket::RawData(buf))
            .map_err(|e| {
                PacketError::SendError(format!(
                    "Failed to send packet to client {}: {}",
                    self.id, e
                ))
            })?;
        Ok(())
    }

    pub fn send_packet(&self, packet: EncodedPacket) -> Result<(), PacketError> {
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

    /// Starts a task that will send packets to the client from the outgoing packet queue.
    /// This task will run until the client is closed or the cancellation token is cancelled.
    pub fn start_outgoing_packet_task(
        self: &Arc<Self>,
        mut sender_recv: UnboundedReceiver<EnqueuedPacket>,
    ) {
        let cancel_token = self.cancel_token.clone();
        let network_writer = self.network_writer.clone();
        let id = self.id;
        let compression_info = self.compression_info.clone();
        let mut connection_updates_recv = self.connection_updates.subscribe();
        let connection_updated = self.connection_updated.clone();

        self.tasks.spawn(async move {
            loop {
                select! {
                    () = cancel_token.cancelled() => {
                        break;
                    }
                    packet = sender_recv.recv() => {
                        if let Some(packet) = packet {

                            let Some(encoded_packet) = (match packet {
                                EnqueuedPacket::EncodedPacket(packet) => Some(packet),
                                EnqueuedPacket::RawData(packet) => {
                                    EncodedPacket::from_data(packet, compression_info.load())
                                        .await
                                        .ok()
                                }
                            }) else {
                                log::warn!("Failed to convert packet to encoded packet for client {id}");
                                continue;
                            };

                            if let Err(err) = network_writer.lock().await.write_packet(&encoded_packet).await
                            {
                                log::warn!("Failed to send packet to client {id}: {err}");
                                cancel_token.cancel();
                            }

                        } else {
                            log::warn!(
                                "Internal packet_sender_recv channel closed for client {id}",
                            );
                            cancel_token.cancel();
                        }
                    }
                    connection_update = connection_updates_recv.recv() => {
                        match connection_update {
                            Ok(connection_update) => {
                                if let ConnectionUpdate::EnableEncryption(key) = connection_update {
                                    network_writer.lock().await.set_encryption(&key);
                                    connection_updated.notify_waiters();
                                }
                            }
                            Err(err) => {
                                if err != RecvError::Closed {
                                    log::warn!("Internal connection_updates_recv channel closed for client {id}: {err}");
                                }
                                cancel_token.cancel();
                            }
                        }
                    }
                }
            }
        });
    }

    pub fn start_incoming_packet_task(
        self: &Arc<Self>,
        mut net_reader: TCPNetworkDecoder<BufReader<OwnedReadHalf>>,
    ) {
        let cancel_token = self.cancel_token.clone();
        let id = self.id;
        let mut connection_updates_recv = self.connection_updates.subscribe();
        let connection_updated = self.connection_updated.clone();

        let self_clone = self.clone();

        self.tasks.spawn(async move {
            loop {
                select! {
                    () = cancel_token.cancelled() => {
                        break;
                    }
                    packet = net_reader.get_raw_packet() => {
                        match packet {
                            Ok(packet) => {
                                //log::info!("Received packet: {:?}, protocol: {:?}", packet.id, connection_protocol.load());
                                if let Err(err) = self_clone.process_packet(packet).await {
                                    log::warn!(
                                        "Failed to get packet from client {id}: {err}",
                                    );
                                }
                            }
                            Err(err) => {
                                log::info!("Failed to get raw packet from client {id}: {err}");
                                cancel_token.cancel();
                            }
                        }
                    }
                    connection_update = connection_updates_recv.recv() => {

                        match connection_update {
                            Ok(connection_update) => {
                                match connection_update {
                                    ConnectionUpdate::EnableEncryption(key) => {
                                        net_reader.set_encryption(&key);
                                    }
                                    ConnectionUpdate::EnableCompression(compression) => {
                                        net_reader.set_compression(compression.threshold);
                                        connection_updated.notify_waiters();
                                    }
                                }
                            }
                            Err(err) => {
                                if err != RecvError::Closed {
                                    log::warn!("Internal connection_updates_recv channel closed for client {id}: {err}");
                                }
                                cancel_token.cancel();
                            }
                        }
                    }
                }
            }
        });
    }

    async fn process_packet(self: &Arc<Self>, packet: RawPacket) -> Result<(), PacketError> {
        match self.protocol.load() {
            ConnectionProtocol::Handshake => self.handle_handshake(packet),
            ConnectionProtocol::Status => self.handle_status(packet).await,
            ConnectionProtocol::Login => self.handle_login(packet).await,
            ConnectionProtocol::Config => self.handle_config(packet).await,
            ConnectionProtocol::Play => self.handle_play(packet),
        }
    }

    pub fn handle_handshake(&self, packet: RawPacket) -> Result<(), PacketError> {
        let data = &mut Cursor::new(packet.payload);

        match packet.id {
            handshake::S_INTENTION => {
                let intent = match SClientIntention::read_packet(data)?.intention {
                    ClientIntent::STATUS => ConnectionProtocol::Status,
                    ClientIntent::LOGIN | ClientIntent::TRANSFER => ConnectionProtocol::Login,
                };
                self.protocol.store(intent);

                if intent != ConnectionProtocol::Status {
                    //TODO: Handle client version being too low or high
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    pub async fn handle_status(&self, packet: RawPacket) -> Result<(), PacketError> {
        let data = &mut Cursor::new(packet.payload);

        match packet.id {
            status::S_STATUS_REQUEST => {
                handle_status_request(self, SStatusRequest::read_packet(data)?).await;
            }
            status::S_PING_REQUEST => {
                handle_ping_request(self, SPingRequest::read_packet(data)?).await;
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    pub async fn handle_login(&self, packet: RawPacket) -> Result<(), PacketError> {
        let data = &mut Cursor::new(packet.payload);

        match packet.id {
            login::S_HELLO => self.handle_hello(SHello::read_packet(data)?).await,
            login::S_KEY => self.handle_key(SKey::read_packet(data)?).await,
            login::S_LOGIN_ACKNOWLEDGED => {
                self.handle_login_acknowledged(SLoginAcknowledged::read_packet(data)?)
                    .await;
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    pub async fn handle_config(&self, packet: RawPacket) -> Result<(), PacketError> {
        let data = &mut Cursor::new(packet.payload);

        match packet.id {
            config::S_CUSTOM_PAYLOAD => {
                self.handle_config_custom_payload(SCustomPayload::read_packet(data)?);
            }
            config::S_CLIENT_INFORMATION => {
                self.handle_client_information(SClientInformation::read_packet(data)?);
            }
            config::S_SELECT_KNOWN_PACKS => {
                self.handle_select_known_packs(SSelectKnownPacks::read_packet(data)?)
                    .await;
            }
            config::S_FINISH_CONFIGURATION => {
                self.handle_finish_configuration(SFinishConfiguration::read_packet(data)?)
                    .await;
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    pub fn handle_play(&self, packet: RawPacket) -> Result<(), PacketError> {
        let data = &mut Cursor::new(packet.payload);

        match packet.id {
            play::C_CUSTOM_PAYLOAD => {
                self.handle_custom_payload(SCustomPayload::read_packet(data)?);
            }
            play::S_CLIENT_TICK_END => {
                self.handle_client_tick_end(SClientTickEnd::read_packet(data)?);
            }
            id => log::info!("play packet id {id} is not known"),
        }
        Ok(())
    }
}

impl JavaTcpClient {
    pub async fn kick(&self, reason: TextComponent) {
        match self.protocol.load() {
            ConnectionProtocol::Login => {
                let packet = CLoginDisconnect::new(reason);
                self.send_bare_packet_now(packet).await;
            }
            ConnectionProtocol::Play | ConnectionProtocol::Config => {
                let packet = CDisconnect::new(reason);
                self.send_bare_packet_now(packet).await;
            }
            _ => {}
        }
        log::debug!("Closing connection for {}", self.id);
        self.close();
    }
}
