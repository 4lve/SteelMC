use std::io::Write;

use crate::{
    packet_traits::{PacketWrite, PrefixedWrite},
    utils::PacketWriteError,
};

impl PrefixedWrite for String {
    fn write_prefixed_bound<P: TryFrom<usize> + TryInto<usize> + PacketWrite>(
        &self,
        writer: &mut impl Write,
        bound: usize,
    ) -> Result<(), PacketWriteError>
    where
        Self: Sized,
    {
        if self.len() > bound {
            return Err(PacketWriteError::TooLong(self.len()));
        }

        let len: P = self
            .len()
            .try_into()
            .map_err(|_| PacketWriteError::Message("This cant fail!".to_string()))?;
        len.write_packet(writer)?;

        writer
            .write_all(self.as_bytes())
            .map_err(PacketWriteError::from)
    }
}
