use bytes::Buf;
use steel_registry::packets::serverbound::{handshake, status};

use crate::{
    packet_traits::PacketRead,
    packets::{
        handshake::ClientIntentionPacket,
        status::{
            s_ping_request_packet::SPingRequestPacket,
            s_status_request_packet::SStatusRequestPacket,
        },
    },
    utils::{ConnectionProtocol, PacketError, RawPacket},
};

/*
When adding a common packet search up .addPacket(CommonPacketTypes.CLIENTBOUND_DISCONNECT) for example to see all it's usages.
*/

// Serverbound packets

#[derive(Clone, Debug)]
pub enum ServerBoundHandshake {
    Intention(ClientIntentionPacket),
}

impl ServerBoundHandshake {
    pub fn from_raw_packet(raw_packet: RawPacket) -> Result<Self, PacketError> {
        match raw_packet.id {
            handshake::SERVERBOUND_INTENTION => {
                let packet = ClientIntentionPacket::read_packet(&mut raw_packet.payload.reader())?;
                Ok(Self::Intention(packet))
            }
            _ => Err(PacketError::MalformedValue(format!(
                "Invalid packet id: {}",
                raw_packet.id
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ServerBoundLogin {}

impl ServerBoundLogin {
    pub fn from_raw_packet(raw_packet: RawPacket) -> Result<Self, PacketError> {
        match raw_packet.id {
            _ => Err(PacketError::MalformedValue(format!(
                "Invalid packet id: {}",
                raw_packet.id
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ServerBoundConfiguration {}

impl ServerBoundConfiguration {
    pub fn from_raw_packet(raw_packet: RawPacket) -> Result<Self, PacketError> {
        match raw_packet.id {
            _ => Err(PacketError::MalformedValue(format!(
                "Invalid packet id: {}",
                raw_packet.id
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ServerBoundStatus {
    StatusRequest(SStatusRequestPacket),
    PingRequest(SPingRequestPacket),
}

impl ServerBoundStatus {
    pub fn from_raw_packet(raw_packet: RawPacket) -> Result<Self, PacketError> {
        match raw_packet.id {
            status::SERVERBOUND_STATUS_REQUEST => {
                let packet = SStatusRequestPacket::read_packet(&mut raw_packet.payload.reader())?;
                Ok(Self::StatusRequest(packet))
            }
            status::SERVERBOUND_PING_REQUEST => {
                let packet = SPingRequestPacket::read_packet(&mut raw_packet.payload.reader())?;
                Ok(Self::PingRequest(packet))
            }
            _ => Err(PacketError::MalformedValue(format!(
                "Invalid packet id: {}",
                raw_packet.id
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ServerBoundPlay {}

impl ServerBoundPlay {
    pub fn from_raw_packet(raw_packet: RawPacket) -> Result<Self, PacketError> {
        match raw_packet.id {
            _ => Err(PacketError::MalformedValue(format!(
                "Invalid packet id: {}",
                raw_packet.id
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ServerPacket {
    Handshake(ServerBoundHandshake),
    Status(ServerBoundStatus),
    Login(ServerBoundLogin),
    Configuration(ServerBoundConfiguration),
    Play(ServerBoundPlay),
}

impl ServerPacket {
    pub fn from_raw_packet(
        raw_packet: RawPacket,
        connection_protocol: ConnectionProtocol,
    ) -> Result<Self, PacketError> {
        match connection_protocol {
            ConnectionProtocol::HANDSHAKING => {
                let packet = ServerBoundHandshake::from_raw_packet(raw_packet)?;
                Ok(Self::Handshake(packet))
            }
            ConnectionProtocol::STATUS => {
                let packet = ServerBoundStatus::from_raw_packet(raw_packet)?;
                Ok(Self::Status(packet))
            }
            ConnectionProtocol::LOGIN => {
                let packet = ServerBoundLogin::from_raw_packet(raw_packet)?;
                Ok(Self::Login(packet))
            }
            ConnectionProtocol::CONFIGURATION => {
                let packet = ServerBoundConfiguration::from_raw_packet(raw_packet)?;
                Ok(Self::Configuration(packet))
            }
            ConnectionProtocol::PLAY => {
                let packet = ServerBoundPlay::from_raw_packet(raw_packet)?;
                Ok(Self::Play(packet))
            }
        }
    }
}
