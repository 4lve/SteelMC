use std::io;

use crate::utils::{PacketReadError, PacketWriteError};

const DEFAULT_BOUND: usize = i32::MAX as _;

// These are the network read/write traits
pub trait PacketRead: Read {
    fn read_packet(data: &mut impl io::Read) -> Result<Self, PacketReadError> {
        Self::read(data).map_err(PacketReadError::from)
    }
}
pub trait PacketWrite: Write {
    fn write_packet(&self, writer: &mut impl io::Write) -> Result<(), PacketWriteError> {
        self.write(writer).map_err(PacketWriteError::from)
    }
}

// These are the general read/write traits with io::error
// Todo! find a better but not longer name, because this conflicts with io::read/write
pub trait Read: Sized {
    fn read(data: &mut impl io::Read) -> Result<Self, io::Error>;
}
pub trait Write {
    fn write(&self, writer: &mut impl io::Write) -> Result<(), io::Error>;
}

pub trait PrefixedRead: Sized {
    fn read_prefixed_bound<P: TryFrom<usize> + TryInto<usize> + Read>(
        data: &mut impl io::Read,
        bound: usize,
    ) -> Result<Self, io::Error>;

    fn read_prefixed<P: TryFrom<usize> + TryInto<usize> + Read>(
        &self,
        data: &mut impl io::Read,
    ) -> Result<Self, io::Error> {
        Self::read_prefixed_bound::<P>(data, DEFAULT_BOUND)
    }
}

pub trait PrefixedWrite {
    fn write_prefixed_bound<P: TryFrom<usize> + Write>(
        &self,
        writer: &mut impl io::Write,
        bound: usize,
    ) -> Result<(), io::Error>;

    fn write_prefixed<P: TryFrom<usize> + Write>(
        &self,
        writer: &mut impl io::Write,
    ) -> Result<(), io::Error> {
        self.write_prefixed_bound::<P>(writer, DEFAULT_BOUND)
    }
}
