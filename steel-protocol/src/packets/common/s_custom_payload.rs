use std::io::Read;

use steel_macros::PacketRead;
use steel_utils::ResourceLocation;

use crate::packet_traits::ReadFrom;

#[derive(PacketRead, Clone, Debug)]
pub struct SCustomPayloadPacket {
    pub resource_location: ResourceLocation,
    //#[read_as(as = "vec")]
    pub payload: Payload,
}

#[derive(Clone, Debug)]
pub struct Payload(pub Vec<u8>);

impl ReadFrom for Payload {
    fn read(data: &mut impl Read) -> Result<Self, std::io::Error> {
        let mut buf = vec![];
        data.read_to_end(&mut buf)?;
        Ok(Self(buf))
    }
}
