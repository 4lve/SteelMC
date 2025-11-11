use std::io::{Read, Result, Write};

use crate::serial::{PrefixedRead, PrefixedWrite, ReadFrom, WriteTo};

use super::VarInt;

pub struct BitSet(pub Box<[u64]>);

impl ReadFrom for BitSet {
    fn read(data: &mut impl Read) -> Result<Self> {
        Ok(Self(Vec::read_prefixed::<VarInt>(data)?.into_boxed_slice()))
    }
}

impl WriteTo for BitSet {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.0.write_prefixed::<VarInt>(writer)
    }
}
