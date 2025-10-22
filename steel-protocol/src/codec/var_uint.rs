use std::io::{Read, Write};

use crate::codec::errors::{ReadingError, WritingError};
use crate::ser::{NetworkReadExt, NetworkWriteExt};

const MAX_SIZE: usize = 5;

/// Returns the exact number of bytes this VarUInt will write when
/// [`Encode::encode`] is called, assuming no error occurs.
pub fn written_size(int: &u32) -> usize {
    (32 - int.leading_zeros() as usize).max(1).div_ceil(7)
}

pub fn write(int: &u32, write: &mut impl Write) -> Result<(), WritingError> {
    let mut val = *int;
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        write.write_u8(byte)?;
        if val == 0 {
            break;
        }
    }
    Ok(())
}

pub fn read(read: &mut impl Read) -> Result<u32, ReadingError> {
    let mut val = 0;
    for i in 0..MAX_SIZE {
        let byte = read.get_u8()?;
        val |= (u32::from(byte) & 0x7F) << (i * 7);
        if byte & 0x80 == 0 {
            return Ok(val);
        }
    }
    Err(ReadingError::TooLarge("VarUInt".to_string()))
}
