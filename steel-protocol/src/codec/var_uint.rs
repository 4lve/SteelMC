use std::io::{Read, Write};

use crate::{codec::errors::{ReadingError, WritingError}, packet_traits::{WriteTo, ReadFrom}};

pub struct VarUint(pub u32);

impl VarUint {
    const MAX_SIZE: usize = 5;

    /// Returns the exact number of bytes this VarUInt will write when
    /// [`Encode::encode`] is called, assuming no error occurs.
    pub fn written_size(self) -> usize {
        (32 - self.0.leading_zeros() as usize).max(1).div_ceil(7)
    }

    pub fn write(self, write: &mut impl Write) -> Result<(), WritingError> {
        let mut val = self.0;
        loop {
            let mut byte = (val & 0x7F) as u8;
            val >>= 7;
            if val != 0 {
                byte |= 0x80;
            }
            byte.write(write).map_err(|e| WritingError::IoError(e))?;
            if val == 0 {
                break;
            }
        }
        Ok(())
    }

    pub fn read(read: &mut impl Read) -> Result<u32, ReadingError> {
        let mut val = 0;
        for i in 0..Self::MAX_SIZE {
            let byte = u8::read(read).map_err(|e| ReadingError::Message(e.to_string()))?;
            val |= (u32::from(byte) & 0x7F) << (i * 7);
            if byte & 0x80 == 0 {
                return Ok(val);
            }
        }
        Err(ReadingError::TooLarge("VarUInt".to_string()))
    }
}
