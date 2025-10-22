use std::io::{ErrorKind, Read, Write};

use crate::codec::errors::{ReadingError, WritingError};
use crate::ser::{NetworkReadExt, NetworkWriteExt};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const MAX_SIZE: usize = 5;

/// Returns the exact number of bytes this VarInt will write when
/// [`Encode::encode`] is called, assuming no error occurs.
pub fn written_size(int: i32) -> usize {
    match int {
        0 => 1,
        n => (31 - n.leading_zeros() as usize) / 7 + 1,
    }
}

pub fn write(mut val: i32, write: &mut impl Write) -> Result<(), WritingError> {
    loop {
        let b: u8 = val as u8 & 0x7F;
        val >>= 7;
        write.write_u8(if val == 0 { b } else { b | 0x80 })?;
        if val == 0 {
            break;
        }
    }
    Ok(())
}

pub fn read(read: &mut impl Read) -> Result<i32, ReadingError> {
    let mut val = 0;
    for i in 0..MAX_SIZE {
        let byte = read.get_u8()?;
        val |= (i32::from(byte) & 0x7F) << (i * 7);
        if byte & 0x80 == 0 {
            return Ok(val);
        }
    }
    Err(ReadingError::TooLarge("VarInt".to_string()))
}

pub async fn read_async(read: &mut (impl AsyncRead + Unpin)) -> Result<i32, ReadingError> {
    let mut val = 0;
    for i in 0..MAX_SIZE {
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
    mut val: i32,
    write: &mut (impl AsyncWrite + Unpin),
) -> Result<(), WritingError> {
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
