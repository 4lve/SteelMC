#![allow(missing_docs)]
use std::{
    collections::HashMap,
    io::{Result, Write},
};

use simdnbt::{ToNbtTag, owned::NbtTag};
use uuid::Uuid;

use crate::{
    BlockPos, Identifier,
    codec::VarInt,
    serial::{PrefixedWrite, WriteTo},
    text::TextComponent,
};

impl WriteTo for bool {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        u8::from(*self).write(writer)?;
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

impl<T: WriteTo> WriteTo for Option<T> {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        if let Some(value) = self {
            true.write(writer)?;
            value.write(writer)
        } else {
            false.write(writer)
        }
    }
}

impl<T: WriteTo, const N: usize> WriteTo for [T; N] {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        for i in self {
            i.write(writer)?;
        }
        Ok(())
    }
}

impl<T: WriteTo, Z: WriteTo> WriteTo for (T, Z) {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.0.write(writer)?;
        self.1.write(writer)
    }
}

#[allow(missing_docs)]
impl<K: WriteTo, V: WriteTo, S: ::std::hash::BuildHasher> WriteTo for HashMap<K, V, S> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        VarInt(self.len() as i32).write(writer)?;
        for (key, value) in self {
            key.write(writer)?;
            value.write(writer)?;
        }
        Ok(())
    }
}

impl<T: WriteTo> WriteTo for Vec<T> {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.write_prefixed::<VarInt>(writer)
    }
}

impl WriteTo for BlockPos {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.as_i64().write(writer)
    }
}

impl WriteTo for TextComponent {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        WriteTo::write(&self.clone().to_nbt_tag(), writer)?;
        Ok(())
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

impl WriteTo for Identifier {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.to_string().write_prefixed::<VarInt>(writer)?;
        Ok(())
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
