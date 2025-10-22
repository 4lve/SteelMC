use std::io::{Read, Write};

use crate::{
    ser::{NetworkReadExt, NetworkWriteExt},
    utils::{PacketReadError, PacketWriteError},
};

const DEFAULT_BOUND: usize = i32::MAX as _;

pub trait PacketRead {
    fn read_packet(data: &mut impl Read) -> Result<Self, PacketReadError>
    where
        Self: Sized;
}
pub trait PacketWrite {
    fn write_packet(&self, writer: &mut impl Write) -> Result<(), PacketWriteError>;
}

pub trait PrefixedRead {
    fn read_prefixed_bound<P: TryFrom<usize> + TryInto<usize> + PacketRead>(
        data: &mut impl Read,
        bound: usize,
    ) -> Result<Self, PacketReadError>
    where
        Self: Sized;

    fn read_prefixed<P: TryFrom<usize> + TryInto<usize> + PacketRead>(
        &self,
        data: &mut impl Read,
    ) -> Result<Self, PacketReadError>
    where
        Self: Sized,
    {
        Self::read_prefixed_bound::<P>(data, DEFAULT_BOUND)
    }
}

pub trait PrefixedWrite {
    fn write_prefixed_bound<P: TryFrom<usize> + TryInto<usize> + PacketWrite>(
        &self,
        writer: &mut impl Write,
        bound: usize,
    ) -> Result<(), PacketWriteError>
    where
        Self: Sized;

    fn write_prefixed<P: TryFrom<usize> + TryInto<usize> + PacketWrite>(
        &self,
        writer: &mut impl Write,
    ) -> Result<(), PacketWriteError>
    where
        Self: Sized,
    {
        self.write_prefixed_bound::<P>(writer, DEFAULT_BOUND)
    }
}

impl PacketRead for i32 {
    fn read_packet(data: &mut impl Read) -> Result<Self, PacketReadError> {
        Ok(data.get_i32_be()?)
    }
}

impl PacketRead for u16 {
    fn read_packet(data: &mut impl Read) -> Result<Self, PacketReadError> {
        Ok(data.get_u16_be()?)
    }
}

impl PacketRead for u8 {
    fn read_packet(data: &mut impl Read) -> Result<Self, PacketReadError> {
        Ok(data.get_u8()?)
    }
}

impl PacketWrite for i32 {
    fn write_packet(&self, writer: &mut impl Write) -> Result<(), PacketWriteError> {
        writer.write_i32_be(*self)?;
        Ok(())
    }
}

impl PacketWrite for i64 {
    fn write_packet(&self, writer: &mut impl Write) -> Result<(), PacketWriteError> {
        writer.write_i64_be(*self)?;
        Ok(())
    }
}

impl PacketWrite for String {
    fn write_packet(&self, writer: &mut impl Write) -> Result<(), PacketWriteError> {
        writer.write_string(self)?;
        Ok(())
    }
}

impl PacketWrite for f32 {
    fn write_packet(&self, writer: &mut impl Write) -> Result<(), PacketWriteError> {
        writer.write_f32_be(*self)?;
        Ok(())
    }
}
