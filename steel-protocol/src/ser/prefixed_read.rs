use std::io;

use crate::packet_traits::{PrefixedRead, Read};

impl PrefixedRead for String {
    fn read_prefixed_bound<P: TryFrom<usize> + TryInto<usize> + Read>(
        data: &mut impl io::Read,
        bound: usize,
    ) -> Result<Self, io::Error> {
        let len: usize = P::read(data)?
            .try_into()
            .map_err(|_| io::Error::other("Invalid Prefix"))?;

        if len > bound {
            return Result::Err(io::Error::other("To long"));
        }

        let mut buf = vec![0; len];
        data.read_exact(&mut buf)?;
        Ok(unsafe { String::from_utf8_unchecked(buf) })
    }
}

impl PrefixedRead for Vec<u8> {
    fn read_prefixed_bound<P: TryFrom<usize> + TryInto<usize> + Read>(
        data: &mut impl io::Read,
        bound: usize,
    ) -> Result<Self, io::Error> {
        let len: usize = P::read(data)?
            .try_into()
            .map_err(|_| io::Error::other("Invalid Prefix"))?;
        if len > bound {
            return Result::Err(io::Error::other("To long"));
        }
        let mut buf = vec![0; len];
        data.read_exact(&mut buf)?;
        Ok(buf)
    }
}
