use std::io::Read;

use crate::{
    packet_traits::{PacketRead, PrefixedRead},
    utils::PacketReadError,
};

impl PrefixedRead for String {
    fn read_prefixed_bound<P: TryFrom<usize> + TryInto<usize> + PacketRead>(
        data: &mut impl Read,
        bound: usize,
    ) -> Result<Self, PacketReadError>
    where
        Self: Sized,
    {
        let len: usize = P::read_packet(data)?
            .try_into()
            .map_err(|_| PacketReadError::MalformedValue("String prefix".to_string()))?;
        if len > bound {
            return Result::Err(PacketReadError::TooLong);
        }
        let mut string = String::with_capacity(len);
        data.read_to_string(&mut string)
            .map_err(|_| PacketReadError::OutOfBounds)?;
        Ok(string)
    }
}
