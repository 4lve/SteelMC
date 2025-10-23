use std::io::{Error, Write};

use crate::packet_traits::WriteTo;

impl WriteTo for u8 {
    fn write(&self, writer: &mut impl Write) -> Result<(), Error> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl WriteTo for u16 {
    fn write(&self, writer: &mut impl Write) -> Result<(), Error> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl WriteTo for u32 {
    fn write(&self, writer: &mut impl Write) -> Result<(), Error> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl WriteTo for u64 {
    fn write(&self, writer: &mut impl Write) -> Result<(), Error> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl WriteTo for i8 {
    fn write(&self, writer: &mut impl Write) -> Result<(), Error> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl WriteTo for i16 {
    fn write(&self, writer: &mut impl Write) -> Result<(), Error> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl WriteTo for i32 {
    fn write(&self, writer: &mut impl Write) -> Result<(), Error> {
        writer.write_all(&self.to_be_bytes())
    }
}

impl WriteTo for i64 {
    fn write(&self, writer: &mut impl Write) -> Result<(), Error> {
        writer.write_all(&self.to_be_bytes())
    }
}
