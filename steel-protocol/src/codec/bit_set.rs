use std::io::{Read, Write};

use crate::packet_traits::{PrefixedRead, PrefixedWrite, ReadFrom, WriteTo};

use super::VarInt;

pub struct BitSet(pub Box<[i64]>);

impl ReadFrom for BitSet {
    fn read(data: &mut impl Read) -> Result<Self, std::io::Error> {
        Ok(Self(Vec::read_prefixed::<VarInt>(data)?.into_boxed_slice()))
    }
}

impl WriteTo for BitSet {
    fn write(&self, writer: &mut impl Write) -> Result<(), std::io::Error> {
        self.0.write_prefixed::<VarInt>(writer)
    }
}
