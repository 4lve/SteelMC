use std::io::{Error, Read, Write};

use crate::packet_traits::{ReadFrom, WriteTo};

use super::VarInt;

pub struct BitSet(pub Box<[i64]>);

impl ReadFrom for BitSet {
    fn read(read: &mut impl Read) -> Result<Self, Error> {
        let length = VarInt::read(read)?;
        let mut array = Vec::with_capacity(length.0 as usize);
        for _ in 0..length.0 {
            array.push(i64::read(read)?);
        }
        Ok(Self(array.into_boxed_slice()))
    }
}

impl WriteTo for BitSet {
    fn write(&self, writer: &mut impl Write) -> Result<(), Error> {
        VarInt(
            self.0
                .len()
                .try_into()
                .map_err(|_| Error::other("BitSet length not representable as VarInt"))?,
        )
        .write(writer)?;

        for &long in self.0.iter() {
            long.write(writer)?;
        }

        Ok(())
    }
}
