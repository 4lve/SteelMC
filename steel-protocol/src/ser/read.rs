use std::io::{Error, Read};

use crate::packet_traits::ReadFrom;

impl ReadFrom for u8 {
    fn read(data: &mut impl Read) -> Result<Self, Error> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for u16 {
    fn read(data: &mut impl Read) -> Result<Self, Error> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for u32 {
    fn read(data: &mut impl Read) -> Result<Self, Error> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for u64 {
    fn read(data: &mut impl Read) -> Result<Self, Error> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for i8 {
    fn read(data: &mut impl Read) -> Result<Self, Error> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for i16 {
    fn read(data: &mut impl Read) -> Result<Self, Error> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for i32 {
    fn read(data: &mut impl Read) -> Result<Self, Error> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for i64 {
    fn read(data: &mut impl Read) -> Result<Self, Error> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for f32 {
    fn read(data: &mut impl Read) -> Result<Self, Error> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl ReadFrom for f64 {
    fn read(data: &mut impl Read) -> Result<Self, Error> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}
