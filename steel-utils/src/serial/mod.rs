use std::io::{Read, Result, Write};

pub mod prefixed_read;
pub mod prefixed_write;
pub mod read;
pub mod write;

const DEFAULT_BOUND: usize = i32::MAX as _;

pub trait ReadFrom: Sized {
    fn read(data: &mut impl Read) -> Result<Self>;
}

pub trait WriteTo {
    fn write(&self, writer: &mut impl Write) -> Result<()>;
}

pub trait PrefixedRead: Sized {
    fn read_prefixed_bound<P: TryInto<usize> + ReadFrom>(
        data: &mut impl Read,
        bound: usize,
    ) -> Result<Self>;

    fn read_prefixed<P: TryInto<usize> + ReadFrom>(data: &mut impl Read) -> Result<Self> {
        Self::read_prefixed_bound::<P>(data, DEFAULT_BOUND)
    }
}

pub trait PrefixedWrite {
    fn write_prefixed_bound<P: TryFrom<usize> + WriteTo>(
        &self,
        writer: &mut impl Write,
        bound: usize,
    ) -> Result<()>;

    fn write_prefixed<P: TryFrom<usize> + WriteTo>(&self, writer: &mut impl Write) -> Result<()> {
        self.write_prefixed_bound::<P>(writer, DEFAULT_BOUND)
    }
}
