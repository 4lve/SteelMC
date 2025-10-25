use std::io::{Error, Read, Result};

use crate::packet_traits::{PrefixedRead, ReadFrom};

impl PrefixedRead for String {
    fn read_prefixed_bound<P: TryInto<usize> + ReadFrom>(
        data: &mut impl Read,
        bound: usize,
    ) -> Result<Self> {
        let len: usize = P::read(data)?
            .try_into()
            .map_err(|_| Error::other("Invalid Prefix"))?;

        if len > bound {
            Err(Error::other("To long"))?
        }

        let mut buf = vec![0; len];
        data.read_exact(&mut buf)?;
        Ok(unsafe { String::from_utf8_unchecked(buf) })
    }
}

impl<T: ReadFrom> PrefixedRead for Vec<T> {
    fn read_prefixed_bound<P: TryInto<usize> + ReadFrom>(
        data: &mut impl Read,
        bound: usize,
    ) -> Result<Self> {
        let len: usize = P::read(data)?
            .try_into()
            .map_err(|_| Error::other("Invalid Prefix"))?;

        if len > bound {
            Err(Error::other("To long"))?
        }
        let mut items = Vec::with_capacity(len);
        for _ in 0..len {
            items.push(T::read(data)?);
        }
        Ok(items)
    }
}
