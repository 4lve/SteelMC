use std::io::{Error, Write};

use crate::packet_traits::{PrefixedWrite, WriteTo};

impl PrefixedWrite for String {
    fn write_prefixed_bound<P: TryFrom<usize> + WriteTo>(
        &self,
        writer: &mut impl Write,
        bound: usize,
    ) -> Result<(), Error> {
        if self.len() > bound {
            return Err(Error::other("To long"));
        }

        let len: P = self
            .len()
            .try_into()
            .map_err(|_| Error::other("This cant happen"))?;
        len.write(writer)?;

        writer.write_all(self.as_bytes())
    }
}

impl PrefixedWrite for Vec<u8> {
    fn write_prefixed_bound<P: TryFrom<usize> + WriteTo>(
        &self,
        writer: &mut impl Write,
        bound: usize,
    ) -> Result<(), Error> {
        if self.len() > bound {
            return Err(Error::other("To long"));
        }

        let len: P = self
            .len()
            .try_into()
            .map_err(|_| Error::other("This cant happen"))?;

        len.write(writer)?;

        writer.write_all(self)
    }
}
