use std::io::{Error, Read};

use crate::packet_traits::{PrefixedRead, ReadFrom};

impl PrefixedRead for String {
    fn read_prefixed_bound<P: TryInto<usize> + ReadFrom>(
        data: &mut impl Read,
        bound: usize,
    ) -> Result<Self, Error> {
        let len: usize = P::read(data)?
            .try_into()
            .map_err(|_| Error::other("Invalid Prefix"))?;

        if len > bound {
            return Result::Err(Error::other("To long"));
        }

        let mut buf = vec![0; len];
        data.read_exact(&mut buf)?;
        Ok(unsafe { String::from_utf8_unchecked(buf) })
    }
}

impl PrefixedRead for Vec<u8> {
    fn read_prefixed_bound<P: TryInto<usize> + ReadFrom>(
        data: &mut impl Read,
        bound: usize,
    ) -> Result<Self, Error> {
        let len: usize = P::read(data)?
            .try_into()
            .map_err(|_| Error::other("Invalid Prefix"))?;

        if len > bound {
            return Result::Err(Error::other("To long"));
        }
        let mut buf = vec![0; len];
        data.read_exact(&mut buf)?;
        Ok(buf)
    }
}
