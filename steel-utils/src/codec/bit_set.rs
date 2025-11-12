use std::io::{Read, Result, Write};

use crate::serial::{PrefixedRead, PrefixedWrite, ReadFrom, WriteTo};

use super::VarInt;

/// A simple bit set implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitSet(pub Box<[u64]>);

#[allow(missing_docs)]
impl ReadFrom for BitSet {
    fn read(data: &mut impl Read) -> Result<Self> {
        Ok(Self(Vec::read_prefixed::<VarInt>(data)?.into_boxed_slice()))
    }
}

#[allow(missing_docs)]
impl WriteTo for BitSet {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.0.write_prefixed::<VarInt>(writer)
    }
}
