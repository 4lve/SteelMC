use std::io;

use crate::packet_traits::Read;



impl Read for u8 {
    fn read(data: &mut impl io::Read) -> Result<Self, io::Error> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Read for u16 {
    fn read(data: &mut impl io::Read) -> Result<Self, io::Error> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Read for u32 {
    fn read(data: &mut impl io::Read) -> Result<Self, io::Error> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl Read for u64 {
    fn read(data: &mut impl io::Read) -> Result<Self, io::Error> {
        let mut buf = [0; size_of::<Self>()];
        data.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
}