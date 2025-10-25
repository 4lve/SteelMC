use std::io::{Cursor, Error, Read, Write};

use crate::packet_traits::{ReadFrom, WriteTo};
use steel_utils::FrontVec;
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

    pub async fn read_async(read: &mut (impl AsyncRead + Unpin)) -> Result<i32, Error> {
        let mut val = 0;
        for i in 0..Self::MAX_SIZE {
            let byte = read
                .read_u8()
                .await
                .map_err(|err| Error::new(err.kind(), "VarInt"))?;
            val |= (i32::from(byte) & 0x7F) << (i * 7);
            if byte & 0x80 == 0 {
                return Ok(val);
            }
        }
        Err(Error::other("VarInt"))
    }

    pub async fn write_async(self, write: &mut (impl AsyncWrite + Unpin)) -> Result<(), Error> {
        let mut val = self.0;
        loop {
            let b: u8 = (val as u8) & 0b01111111;
            val >>= 7;
            write
                .write_u8(if val == 0 { b } else { b | 0b10000000 })
                .await?;
            if val == 0 {
                break;
            }
        }
        Ok(())
    }

    // We could just get the writen size in place,
    // but in our use case its already calculated
    pub fn set_in_front(&self, vec: &mut FrontVec, varint_size: usize) {
        // No heap allocation :)
        let mut buf = [0; Self::MAX_SIZE];
        self.write(&mut Cursor::new(&mut buf[..])).unwrap();
        vec.set_in_front(&buf[..varint_size]);
    }
}

impl ReadFrom for VarInt {
    fn read(read: &mut impl Read) -> Result<Self, Error> {
        let mut val = 0;
        for i in 0..Self::MAX_SIZE {
            let byte = u8::read(read)?;
            val |= (i32::from(byte) & 0x7F) << (i * 7);
            if byte & 0x80 == 0 {
                return Ok(Self(val));
            }
        }
        Err(Error::other("VarInt to long"))
    }
}

impl WriteTo for VarInt {
    fn write(&self, writer: &mut impl Write) -> Result<(), Error> {
        let mut val = self.0;
        loop {
            let b: u8 = val as u8 & 0x7F;
            val >>= 7;
            if val == 0 {
                b.write(writer)?;
                break;
            } else {
                (b | 0x80).write(writer)?;
            }
        }
        Ok(())
    }
}

impl From<usize> for VarInt {
    fn from(value: usize) -> Self {
        Self(value as _)
    }
}

impl From<VarInt> for usize {
    fn from(value: VarInt) -> usize {
        value.0 as _
    }
}

impl From<i32> for VarInt {
    fn from(value: i32) -> Self {
        Self(value as _)
    }
}

impl From<VarInt> for i32 {
    fn from(value: VarInt) -> i32 {
        value.0
    }
}
