use std::io::{Error, Read, Write};

use crate::utils::PacketError;

const DEFAULT_BOUND: usize = i32::MAX as _;

// These are the network read/write traits
pub trait PacketRead: ReadFrom {
    fn read_packet(data: &mut impl Read) -> Result<Self, PacketError> {
        Self::read(data).map_err(PacketError::from)
    }
}
pub trait PacketWrite: WriteTo {
    fn write_packet(&self, writer: &mut impl Write) -> Result<(), PacketError> {
        self.write(writer).map_err(PacketError::from)
    }
}

// These are the general read/write traits with io::error
// We dont use Write/Read because it conflicts with std::io::Read/Write
pub trait ReadFrom: Sized {
    fn read(data: &mut impl Read) -> Result<Self, Error>;
}
pub trait WriteTo {
    fn write(&self, writer: &mut impl Write) -> Result<(), Error>;
}

pub trait PrefixedRead: Sized {
    fn read_prefixed_bound<P: TryInto<usize> + ReadFrom>(
        data: &mut impl Read,
        bound: usize,
    ) -> Result<Self, Error>;

    fn read_prefixed<P: TryInto<usize> + ReadFrom>(
        &self,
        data: &mut impl Read,
    ) -> Result<Self, Error> {
        Self::read_prefixed_bound::<P>(data, DEFAULT_BOUND)
    }
}

pub trait PrefixedWrite {
    fn write_prefixed_bound<P: TryFrom<usize> + WriteTo>(
        &self,
        writer: &mut impl Write,
        bound: usize,
    ) -> Result<(), Error>;

    fn write_prefixed<P: TryFrom<usize> + WriteTo>(
        &self,
        writer: &mut impl Write,
    ) -> Result<(), Error> {
        self.write_prefixed_bound::<P>(writer, DEFAULT_BOUND)
    }
}
