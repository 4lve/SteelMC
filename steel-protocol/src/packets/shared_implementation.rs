use std::io::{self, Write, Result};

use steel_utils::text::TextComponentBase;
use uuid::Uuid;

use crate::packet_traits::{ReadFrom, WriteTo};

impl WriteTo for TextComponentBase {
    fn write(&self, _: &mut impl Write) -> Result<()> {
        //TODO: Implement
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
