use std::io::{Error, Write, Result};

use crate::packet_traits::{PrefixedWrite, WriteTo};

impl PrefixedWrite for String {
    fn write_prefixed_bound<P: TryFrom<usize> + WriteTo>(
        &self,
        writer: &mut impl Write,
        bound: usize,
    ) -> Result<()> {
        if self.len() > bound {
            Err(Error::other("To long"))?
        }

        let len: P = self
            .len()
            .try_into()
            .map_err(|_| Error::other("This cant happen"))?;
        len.write(writer)?;

        writer.write_all(self.as_bytes())
    }
}

impl<T: WriteTo> PrefixedWrite for Vec<T> {
    fn write_prefixed_bound<P: TryFrom<usize> + WriteTo>(
        &self,
        writer: &mut impl Write,
        bound: usize,
    ) -> Result<()> {
        if self.len() > bound {
            Err(Error::other("To long"))?
        }

        let len: P = self
            .len()
            .try_into()
            .map_err(|_| Error::other("This cant happen"))?;

        len.write(writer)?;

        for property in self {
            property.write(writer)?;
        }

        Ok(())
    }
}
