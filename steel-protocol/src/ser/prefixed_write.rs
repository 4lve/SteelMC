use std::io;

use crate::packet_traits::{PrefixedWrite, Write};

impl PrefixedWrite for String {
    fn write_prefixed_bound<P: TryFrom<usize> + Write>(
        &self,
        writer: &mut impl io::Write,
        bound: usize,
    ) -> Result<(), io::Error> {
        if self.len() > bound {
            return Err(io::Error::other("To long"));
        }

        let len: P = self
            .len()
            .try_into()
            .map_err(|_| io::Error::other("This cant happen"))?;
        len.write(writer)?;

        writer.write_all(self.as_bytes())
    }
}

impl PrefixedWrite for Vec<u8> {
    fn write_prefixed_bound<P: TryFrom<usize> + Write>(
        &self,
        writer: &mut impl io::Write,
        bound: usize,
    ) -> Result<(), io::Error> {
        if self.len() > bound {
            return Err(io::Error::other("To long"));
        }

        let len: P = self
            .len()
            .try_into()
            .map_err(|_| io::Error::other("This cant happen"))?;

        len.write(writer)?;

        writer.write_all(&self)
    }
}
