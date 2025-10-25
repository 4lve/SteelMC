use std::io::{Write, Result};

use crate::{
    codec::VarInt,
    packet_traits::{PrefixedWrite, WriteTo},
};

impl WriteTo for bool {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        (*self as u8).write(writer)?;
        Ok(())
    }
}

impl WriteTo for u8 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl WriteTo for u16 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl WriteTo for u32 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl WriteTo for u64 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl WriteTo for i8 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl WriteTo for i16 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl WriteTo for i32 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl WriteTo for i64 {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl WriteTo for Option<String> {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        if let Some(value) = self {
            (true).write(writer)?;
            value.write_prefixed::<VarInt>(writer)?;
        } else {
            (false).write(writer)?;
        }
        Ok(())
    }
}
