use std::io::{Read, Result};

use crate::packet_traits::ReadFrom;

impl ReadFrom for bool {
    fn read(data: &mut impl Read) -> Result<Self> {
        let byte = u8::read(data)?;
        Ok(byte == 1)
    }
}

impl ReadFrom for u8 {
    fn read(data: &mut impl Read) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for u16 {
    fn read(data: &mut impl Read) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for u32 {
    fn read(data: &mut impl Read) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for u64 {
    fn read(data: &mut impl Read) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for i8 {
    fn read(data: &mut impl Read) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for i16 {
    fn read(data: &mut impl Read) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for i32 {
    fn read(data: &mut impl Read) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for i64 {
    fn read(data: &mut impl Read) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for f32 {
    fn read(data: &mut impl Read) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for f64 {
    fn read(data: &mut impl Read) -> Result<Self> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}
