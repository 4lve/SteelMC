use std::io::{self, Error, ErrorKind, Read, Write};

use crate::codec::errors::{ReadingError, WritingError};
use crate::packet_traits::{ReadFrom, WriteTo};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub struct VarInt(pub i32);

impl VarInt {
    pub const MAX_SIZE: usize = 5;

    /// Returns the exact number of bytes this VarInt will write when
    /// [`Encode::encode`] is called, assuming no error occurs.
    pub fn written_size(val: i32) -> usize {
        match val {
            0 => 1,
            n => (31 - n.leading_zeros() as usize) / 7 + 1,
        }
    }

    pub fn write(self, writer: &mut impl Write) -> Result<(), WritingError> {
        let mut val = self.0;
        loop {
            let b: u8 = val as u8 & 0x7F;
            val >>= 7;
            if val == 0 {
                b.write(writer).map_err(|e| WritingError::IoError(e))?;
                break;
            } else {
                (b | 0x80)
                    .write(writer)
                    .map_err(|e| WritingError::IoError(e))?;
            }
            if val == 0 {
                break;
            }
        }
        Ok(())
    }

    pub fn read(read: &mut impl Read) -> Result<i32, Error> {
        let mut val = 0;
        for i in 0..Self::MAX_SIZE {
            let byte = u8::read(read)?;
            val |= (i32::from(byte) & 0x7F) << (i * 7);
            if byte & 0x80 == 0 {
                return Ok(val);
            }
        }
        Err(io::Error::other("VarInt"))
    }

    pub async fn read_async(read: &mut (impl AsyncRead + Unpin)) -> Result<i32, ReadingError> {
        let mut val = 0;
        for i in 0..Self::MAX_SIZE {
            let byte = read.read_u8().await.map_err(|err| {
                if i == 0 && matches!(err.kind(), ErrorKind::UnexpectedEof) {
                    ReadingError::CleanEOF("VarInt".to_string())
                } else {
                    ReadingError::Incomplete(err.to_string())
                }
            })?;
            val |= (i32::from(byte) & 0x7F) << (i * 7);
            if byte & 0x80 == 0 {
                return Ok(val);
            }
        }
        Err(ReadingError::TooLarge("VarInt".to_string()))
    }

    pub async fn write_async(
        self,
        write: &mut (impl AsyncWrite + Unpin),
    ) -> Result<(), WritingError> {
        let mut val = self.0;
        loop {
            let b: u8 = (val as u8) & 0b01111111;
            val >>= 7;
            write
                .write_u8(if val == 0 { b } else { b | 0b10000000 })
                .await
                .map_err(WritingError::IoError)?;
            if val == 0 {
                break;
            }
        }
        Ok(())
    }
}

impl ReadFrom for VarInt {
    fn read(_: &mut impl Read) -> Result<Self, Error> {
        unreachable!()
    }
}

impl WriteTo for VarInt {
    fn write(&self, _: &mut impl Write) -> Result<(), Error> {
        unreachable!()
    }
}

impl From<usize> for VarInt {
    fn from(value: usize) -> Self {
        Self(value as _)
    }
}

impl Into<usize> for VarInt {
    fn into(self) -> usize {
        self.0 as _
    }
}

impl From<i32> for VarInt {
    fn from(value: i32) -> Self {
        Self(value as _)
    }
}

impl Into<i32> for VarInt {
    fn into(self) -> i32 {
        self.0 as _
    }
}
