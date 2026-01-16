//! This module contains everything related to text components.
use simdnbt::owned::{NbtCompound, NbtList, NbtTag};
use std::io::{Result as IoResult, Write};
use text_components::{
    TextComponent,
    content::{Content, Resolvable},
    custom::CustomData,
    nbt::NbtBuilder,
    resolving::{NoResolutor, TextResolutor},
};

/// A [TextResolutor] for the console
pub struct DisplayResolutor;
impl TextResolutor for DisplayResolutor {
    fn resolve_content(&self, resolvable: &Resolvable) -> TextComponent {
        TextComponent {
            content: Content::Resolvable(resolvable.clone()),
            ..Default::default()
        }
    }

    fn resolve_custom(&self, _data: &CustomData) -> Option<TextComponent> {
        None
    }

    fn translate(&self, key: &str) -> Option<String> {
        crate::translations_registry::TRANSLATIONS
            .get(key)
            .map(ToString::to_string)
    }
}

/// Encodes the text component to NBT bytes for network transmission.
/// Uses network NBT format: `TAG_Compound` byte, no name, then content.
///
/// # Panics
///
/// Panics if the text component fails to serialize to an NBT compound or if
/// writing the NBT compound to bytes fails.
pub fn encode_text_component(component: &TextComponent) -> Vec<u8> {
    let compound = match component.build(&NoResolutor, NbtBuilder) {
        simdnbt::owned::Nbt::Some(base_nbt) => base_nbt.as_compound(),
        simdnbt::owned::Nbt::None => panic!("TextComponent must serialize to NBT compound"),
    };
    log::debug!("TextComponent NBT tag: {compound:?}");
    let mut buffer = Vec::new();
    // Network NBT format per NbtIo.writeAnyTag: TAG byte + content
    buffer.push(0x0A); // TAG_Compound
    write_nbt_compound(&mut buffer, &compound).expect("Failed to write NBT compound");
    log::debug!(
        "Encoded NBT bytes (len={}): {:02X?}",
        buffer.len(),
        &buffer[..buffer.len().min(50)]
    );
    buffer
}

/// Helper to write NBT compound content
fn write_nbt_compound(writer: &mut Vec<u8>, compound: &NbtCompound) -> std::io::Result<()> {
    for (key, value) in compound.iter() {
        // Write tag type
        writer.write_all(&[get_nbt_tag_id(value)])?;
        // Write key as modified UTF-8 string
        let key_bytes = key.as_bytes();
        writer.write_all(&(key_bytes.len() as u16).to_be_bytes())?;
        writer.write_all(key_bytes)?;
        // Write value payload
        write_nbt_tag_payload(writer, value)?;
    }
    // Write TAG_End
    writer.write_all(&[0x00])?;
    Ok(())
}

fn get_nbt_tag_id(tag: &NbtTag) -> u8 {
    match tag {
        NbtTag::Byte(_) => 0x01,
        NbtTag::Short(_) => 0x02,
        NbtTag::Int(_) => 0x03,
        NbtTag::Long(_) => 0x04,
        NbtTag::Float(_) => 0x05,
        NbtTag::Double(_) => 0x06,
        NbtTag::ByteArray(_) => 0x07,
        NbtTag::String(_) => 0x08,
        NbtTag::List(_) => 0x09,
        NbtTag::Compound(_) => 0x0A,
        NbtTag::IntArray(_) => 0x0B,
        NbtTag::LongArray(_) => 0x0C,
    }
}

fn write_nbt_tag_payload(writer: &mut Vec<u8>, tag: &NbtTag) -> IoResult<()> {
    match tag {
        NbtTag::Byte(v) => writer.write_all(&[*v as u8])?,
        NbtTag::Short(v) => writer.write_all(&v.to_be_bytes())?,
        NbtTag::Int(v) => writer.write_all(&v.to_be_bytes())?,
        NbtTag::Long(v) => writer.write_all(&v.to_be_bytes())?,
        NbtTag::Float(v) => writer.write_all(&v.to_be_bytes())?,
        NbtTag::Double(v) => writer.write_all(&v.to_be_bytes())?,
        NbtTag::ByteArray(v) => {
            writer.write_all(&(v.len() as i32).to_be_bytes())?;
            writer.write_all(v)?;
        }
        NbtTag::String(v) => {
            let bytes = v.as_bytes();
            writer.write_all(&(bytes.len() as u16).to_be_bytes())?;
            writer.write_all(bytes)?;
        }
        NbtTag::List(list) => write_nbt_list(writer, list)?,
        NbtTag::Compound(compound) => write_nbt_compound(writer, compound)?,
        NbtTag::IntArray(v) => {
            writer.write_all(&(v.len() as i32).to_be_bytes())?;
            for int in v {
                writer.write_all(&int.to_be_bytes())?;
            }
        }
        NbtTag::LongArray(v) => {
            writer.write_all(&(v.len() as i32).to_be_bytes())?;
            for long in v {
                writer.write_all(&long.to_be_bytes())?;
            }
        }
    }
    Ok(())
}

fn write_nbt_list(writer: &mut Vec<u8>, list: &NbtList) -> IoResult<()> {
    match list {
        NbtList::Empty => {
            writer.write_all(&[0x00])?; // TAG_End
            writer.write_all(&[0x00, 0x00, 0x00, 0x00])?; // Length 0
        }
        NbtList::Byte(v) => {
            writer.write_all(&[0x01])?;
            writer.write_all(&(v.len() as i32).to_be_bytes())?;
            for b in v {
                writer.write_all(&[*b as u8])?;
            }
        }
        NbtList::Short(v) => {
            writer.write_all(&[0x02])?;
            writer.write_all(&(v.len() as i32).to_be_bytes())?;
            for s in v {
                writer.write_all(&s.to_be_bytes())?;
            }
        }
        NbtList::Int(v) => {
            writer.write_all(&[0x03])?;
            writer.write_all(&(v.len() as i32).to_be_bytes())?;
            for i in v {
                writer.write_all(&i.to_be_bytes())?;
            }
        }
        NbtList::Long(v) => write_nbt_list_long(writer, v)?,
        NbtList::Float(v) => write_nbt_list_float(writer, v)?,
        NbtList::Double(v) => write_nbt_list_double(writer, v)?,
        NbtList::ByteArray(v) => write_nbt_list_byte_array(writer, v)?,
        NbtList::String(v) => write_nbt_list_string(writer, v)?,
        NbtList::List(v) => write_nbt_list_list(writer, v)?,
        NbtList::Compound(v) => write_nbt_list_compound(writer, v)?,
        NbtList::IntArray(v) => write_nbt_list_int_array(writer, v)?,
        NbtList::LongArray(v) => write_nbt_list_long_array(writer, v)?,
    }
    Ok(())
}

fn write_nbt_list_long(writer: &mut Vec<u8>, v: &[i64]) -> IoResult<()> {
    writer.write_all(&[0x04])?;
    writer.write_all(&(v.len() as i32).to_be_bytes())?;
    for l in v {
        writer.write_all(&l.to_be_bytes())?;
    }
    Ok(())
}

fn write_nbt_list_float(writer: &mut Vec<u8>, v: &[f32]) -> IoResult<()> {
    writer.write_all(&[0x05])?;
    writer.write_all(&(v.len() as i32).to_be_bytes())?;
    for f in v {
        writer.write_all(&f.to_be_bytes())?;
    }
    Ok(())
}

fn write_nbt_list_double(writer: &mut Vec<u8>, v: &[f64]) -> IoResult<()> {
    writer.write_all(&[0x06])?;
    writer.write_all(&(v.len() as i32).to_be_bytes())?;
    for d in v {
        writer.write_all(&d.to_be_bytes())?;
    }
    Ok(())
}

fn write_nbt_list_byte_array(writer: &mut Vec<u8>, v: &[Vec<u8>]) -> IoResult<()> {
    writer.write_all(&[0x07])?;
    writer.write_all(&(v.len() as i32).to_be_bytes())?;
    for arr in v {
        writer.write_all(&(arr.len() as i32).to_be_bytes())?;
        writer.write_all(arr)?;
    }
    Ok(())
}

fn write_nbt_list_string(writer: &mut Vec<u8>, v: &[simdnbt::Mutf8String]) -> IoResult<()> {
    writer.write_all(&[0x08])?;
    writer.write_all(&(v.len() as i32).to_be_bytes())?;
    for s in v {
        let bytes = s.as_bytes();
        writer.write_all(&(bytes.len() as u16).to_be_bytes())?;
        writer.write_all(bytes)?;
    }
    Ok(())
}

fn write_nbt_list_list(writer: &mut Vec<u8>, v: &[NbtList]) -> IoResult<()> {
    writer.write_all(&[0x09])?;
    writer.write_all(&(v.len() as i32).to_be_bytes())?;
    for l in v {
        write_nbt_list(writer, l)?;
    }
    Ok(())
}

fn write_nbt_list_compound(writer: &mut Vec<u8>, v: &[NbtCompound]) -> IoResult<()> {
    writer.write_all(&[0x0A])?;
    writer.write_all(&(v.len() as i32).to_be_bytes())?;
    for c in v {
        write_nbt_compound(writer, c)?;
    }
    Ok(())
}

fn write_nbt_list_int_array(writer: &mut Vec<u8>, v: &[Vec<i32>]) -> IoResult<()> {
    writer.write_all(&[0x0B])?;
    writer.write_all(&(v.len() as i32).to_be_bytes())?;
    for arr in v {
        writer.write_all(&(arr.len() as i32).to_be_bytes())?;
        for i in arr {
            writer.write_all(&i.to_be_bytes())?;
        }
    }
    Ok(())
}

fn write_nbt_list_long_array(writer: &mut Vec<u8>, v: &[Vec<i64>]) -> IoResult<()> {
    writer.write_all(&[0x0C])?;
    writer.write_all(&(v.len() as i32).to_be_bytes())?;
    for arr in v {
        writer.write_all(&(arr.len() as i32).to_be_bytes())?;
        for l in arr {
            writer.write_all(&l.to_be_bytes())?;
        }
    }
    Ok(())
}
