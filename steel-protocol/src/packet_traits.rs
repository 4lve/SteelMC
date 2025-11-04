use std::{
    io::{Error, Read, Write},
    sync::Arc,
};

use async_compression::{Level, tokio::write::ZlibEncoder};
use serde::Deserialize;
use steel_utils::FrontVec;
use tokio::io::AsyncWriteExt;

use crate::{
    codec::VarInt,
    utils::{ConnectionProtocol, MAX_PACKET_DATA_SIZE, MAX_PACKET_SIZE, PacketError},
};

const DEFAULT_BOUND: usize = i32::MAX as _;

// These are the network read/write traits
pub trait ServerPacket: ReadFrom {
    fn read_packet(data: &mut impl Read) -> Result<Self, PacketError> {
        Self::read(data).map_err(PacketError::from)
    }
}
pub trait ClientPacket: WriteTo {
    fn write_packet(
        &self,
        writer: &mut impl Write,
        protocol: ConnectionProtocol,
    ) -> Result<(), PacketError> {
        let packet_id = self
            .get_id(protocol)
            .ok_or(PacketError::InvalidProtocol(format!(
                "Invalid protocol {:?}",
                protocol
            )))?;
        VarInt(packet_id).write(writer)?;
        self.write(writer).map_err(PacketError::from)
    }
    fn get_id(&self, protocol: ConnectionProtocol) -> Option<i32>;
}

// These are the general read/write traits with io::error
// We dont use Write/Read because it conflicts with std::io::Read/Write
pub trait ReadFrom: Sized {
    fn read(data: &mut impl Read) -> Result<Self, Error>;
}
pub trait WriteTo {
    fn write(&self, writer: &mut impl Write) -> Result<(), Error>;
}

pub trait PrefixedRead: Sized {
    fn read_prefixed_bound<P: TryInto<usize> + ReadFrom>(
        data: &mut impl Read,
        bound: usize,
    ) -> Result<Self, Error>;

    fn read_prefixed<P: TryInto<usize> + ReadFrom>(data: &mut impl Read) -> Result<Self, Error> {
        Self::read_prefixed_bound::<P>(data, DEFAULT_BOUND)
    }
}

pub trait PrefixedWrite {
    fn write_prefixed_bound<P: TryFrom<usize> + WriteTo>(
        &self,
        writer: &mut impl Write,
        bound: usize,
    ) -> Result<(), Error>;

    fn write_prefixed<P: TryFrom<usize> + WriteTo>(
        &self,
        writer: &mut impl Write,
    ) -> Result<(), Error> {
        self.write_prefixed_bound::<P>(writer, DEFAULT_BOUND)
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct CompressionInfo {
    /// The compression threshold used when compression is enabled.
    pub threshold: usize,
    /// A value between `0..9`.
    /// `1` = Optimize for the best speed of encoding.
    /// `9` = Optimize for the size of data being encoded.
    pub level: i32,
}

impl Default for CompressionInfo {
    fn default() -> Self {
        Self {
            threshold: 256,
            level: 4,
        }
    }
}

/// Represents an encoded clientbound packet, optionally applying compression based on threshold and level.
///
/// # Packet Size Limits
/// - Maximum packet size: 2097151 bytes (2^21 - 1, max 3-byte VarInt)
/// - Maximum uncompressed size for compressed packets: 8388608 bytes (2^23)
/// - Length field must not exceed 3 bytes
///
/// # Packet Encoding Format
///
/// **Without Compression:**
/// ```text
/// [Length: VarInt]     Length of (Packet ID + Data)
/// [Packet ID: VarInt]  Protocol ID from packet report
/// [Data: Byte Array]   Packet payload
/// ```
///
/// **With Compression (size >= threshold):**
/// ```text
/// [Length: VarInt]     Length of (Data Length + compressed data)
/// [Data Length: VarInt] Length of uncompressed (Packet ID + Data)
/// [Compressed Data]    zlib compressed (Packet ID + Data)
/// ```
///
/// **With Compression (size < threshold):**
/// ```text
/// [Length: VarInt]     Length of (Data Length + uncompressed data)
/// [Data Length: VarInt] 0 to indicate uncompressed
/// [Packet ID: VarInt]  Protocol ID from packet report
/// [Data: Byte Array]   Uncompressed packet payload
/// ```
///
/// Compression is only applied when:
/// 1. Compression is enabled via Set Compression packet
/// 2. The uncompressed data length meets/exceeds the threshold
/// 3. The threshold is non-negative
#[derive(Clone)]
pub struct EncodedPacket {
    // This is optimized for reduces allocation
    pub encoded_data: Arc<FrontVec>,
}

impl EncodedPacket {
    fn from_data_no_compression(mut packet_data: FrontVec) -> Result<Self, PacketError> {
        let data_len = packet_data.len();
        let varint_size = VarInt::written_size(data_len as _);

        let complete_len = varint_size + data_len;
        if complete_len > MAX_PACKET_SIZE {
            return Err(PacketError::TooLong(complete_len));
        }

        VarInt(data_len as _).set_in_front(&mut packet_data, varint_size);

        Ok(Self {
            encoded_data: Arc::new(packet_data),
        })
    }

    async fn from_packet_data(
        mut packet_data: FrontVec,
        threshold: usize,
        level: i32,
    ) -> Result<Self, PacketError> {
        let data_len = packet_data.len();
        // We dont need any more size check to convert to i32 as MAX_PACKET_DATA_SIZE < i32::MAX
        if data_len + VarInt::MAX_SIZE * 2 > MAX_PACKET_DATA_SIZE {
            Err(PacketError::TooLong(data_len))?
        }

        if data_len >= threshold {
            let mut buf = FrontVec::new(10);
            let mut compressor = ZlibEncoder::with_quality(&mut buf, Level::Precise(level));

            compressor
                .write_all(&packet_data)
                .await
                .map_err(|e| PacketError::CompressionFailed(e.to_string()))?;
            compressor
                .flush()
                .await
                .map_err(|e| PacketError::CompressionFailed(e.to_string()))?;

            // compressed data cant be larger so we dont need to check the size again
            let varint_size = VarInt::written_size(data_len as _);
            let full_len = varint_size + buf.len();
            let full_varint_size = VarInt::written_size(full_len as _);

            VarInt(data_len as _).set_in_front(&mut buf, varint_size);
            VarInt(full_len as _).set_in_front(&mut buf, full_varint_size);
            log::trace!(
                "data length: {}, full length: {}, varint size: {}, full varint size: {}",
                data_len,
                full_len,
                varint_size,
                full_varint_size
            );

            Ok(Self {
                encoded_data: Arc::new(buf),
            })
        } else {
            // Pushed before data:
            // Length of (Data Length) + length of compressed (Packet ID + Data)
            // 0 to indicate uncompressed

            let data_len_with_header = data_len + 1;
            let varint_size = VarInt::written_size(data_len_with_header as _);

            VarInt(0).set_in_front(&mut packet_data, 1);
            VarInt(data_len_with_header as _).set_in_front(&mut packet_data, varint_size);

            Ok(Self {
                encoded_data: Arc::new(packet_data),
            })
        }
    }

    pub async fn from_packet<P: ClientPacket>(
        packet: P,
        compression_info: Option<CompressionInfo>,
        protocol: ConnectionProtocol,
    ) -> Result<Self, PacketError> {
        let buf = Self::write_vec(packet, protocol)?;
        Self::from_data(buf, compression_info).await
    }

    pub fn write_vec<P: ClientPacket>(
        packet: P,
        protocol: ConnectionProtocol,
    ) -> Result<FrontVec, PacketError> {
        let mut buf = FrontVec::new(6);
        packet.write_packet(&mut buf, protocol)?;
        Ok(buf)
    }

    pub async fn from_data(
        buf: FrontVec,
        compression_info: Option<CompressionInfo>,
    ) -> Result<Self, PacketError> {
        if let Some(compression_info) = compression_info {
            Self::from_packet_data(buf, compression_info.threshold, compression_info.level).await
        } else {
            Self::from_data_no_compression(buf)
        }
    }
}
