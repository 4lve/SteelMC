use std::{
    io::{self, Result, Write},
    str::FromStr,
};

use simdnbt::owned::NbtTag;
use steel_macros::{ReadFrom, WriteTo};
use steel_utils::{ResourceLocation, text::TextComponent};
use uuid::Uuid;

use crate::{
    codec::VarInt,
    packet_traits::{PrefixedRead, PrefixedWrite, ReadFrom, WriteTo},
};

impl WriteTo for TextComponent {
    fn write(&self, _writer: &mut impl Write) -> Result<()> {
        todo!()
    }
}

impl ReadFrom for Uuid {
    fn read(data: &mut impl io::Read) -> Result<Self> {
        let most_significant_bits = u64::read(data)?;
        let least_significant_bits = u64::read(data)?;

        Ok(Uuid::from_u64_pair(
            most_significant_bits,
            least_significant_bits,
        ))
    }
}

impl WriteTo for Uuid {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        let (most_significant_bits, least_significant_bits) = self.as_u64_pair();
        most_significant_bits.write(writer)?;
        least_significant_bits.write(writer)?;
        Ok(())
    }
}

impl ReadFrom for ResourceLocation {
    fn read(data: &mut impl io::Read) -> Result<Self> {
        ResourceLocation::from_str(&String::read_prefixed::<VarInt>(data)?)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }
}

impl WriteTo for ResourceLocation {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.to_string().write_prefixed::<VarInt>(writer)?;
        Ok(())
    }
}

#[derive(Clone, Debug, WriteTo, ReadFrom)]
pub struct KnownPack {
    #[write_as(as = "string")]
    #[read_as(as = "string")]
    pub namespace: String,
    #[write_as(as = "string")]
    #[read_as(as = "string")]
    pub id: String,
    #[write_as(as = "string")]
    #[read_as(as = "string")]
    pub version: String,
}

impl KnownPack {
    pub fn new(namespace: String, id: String, version: String) -> Self {
        Self {
            namespace,
            id,
            version,
        }
    }
}

impl WriteTo for NbtTag {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        let mut buf = Vec::new();
        self.write(&mut buf);
        writer.write_all(&buf)?;
        Ok(())
    }
}
