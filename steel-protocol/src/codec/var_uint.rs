use std::io::{Error, Read, Write};

use crate::packet_traits::{ReadFrom, WriteTo};

pub struct VarUint(pub u32);

impl VarUint {
    const MAX_SIZE: usize = 5;

    /// Returns the exact number of bytes this VarUInt will write when
    /// [`Encode::encode`] is called, assuming no error occurs.
    pub fn written_size(self) -> usize {
        (32 - self.0.leading_zeros() as usize).max(1).div_ceil(7)
    }

    pub fn write(self, writer: &mut impl Write) -> Result<(), Error> {
        let mut val = self.0;
        loop {
            let mut byte = (val & 0x7F) as u8;
            val >>= 7;
            if val != 0 {
                byte |= 0x80;
            }
            byte.write(writer)?;
            if val == 0 {
                break;
            }
        }
        Ok(())
    }

    pub fn read(read: &mut impl Read) -> Result<u32, Error> {
        let mut val = 0;
        for i in 0..Self::MAX_SIZE {
            let byte = u8::read(read)?;
            val |= (u32::from(byte) & 0x7F) << (i * 7);
            if byte & 0x80 == 0 {
                return Ok(val);
            }
        }
        Err(Error::other("Malformed VarUint"))
    }
}
