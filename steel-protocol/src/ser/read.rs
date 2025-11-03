use std::{
    io::{Read, Result},
    mem::{self, MaybeUninit},
};

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

impl<T: ReadFrom> ReadFrom for Option<T> {
    fn read(data: &mut impl Read) -> Result<Self> {
        if bool::read(data)? {
            Ok(Some(T::read(data)?))
        } else {
            Ok(None)
        }
    }
}

impl<T: ReadFrom, const N: usize> ReadFrom for [T; N] {
    fn read(data: &mut impl Read) -> Result<Self> {
        #[allow(clippy::uninit_assumed_init)]
        let mut buf: [T; N] = unsafe { MaybeUninit::uninit().assume_init() };

        for i in &mut buf {
            mem::forget(mem::replace(i, T::read(data)?));
        }

        Ok(buf)
    }
}
