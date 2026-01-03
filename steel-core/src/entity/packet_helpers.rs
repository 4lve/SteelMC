//! Helper functions for converting entity data to packet format

use super::EntityDataValue;
use steel_utils::codec::VarInt;
use steel_utils::serial::{PrefixedWrite, WriteTo};

/// Converts entity data to packet-ready format
pub fn serialize_entity_data_value(value: &EntityDataValue) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();

    match value {
        EntityDataValue::Byte(v) => {
            v.write(&mut bytes)?;
        }
        EntityDataValue::Int(v) => {
            VarInt(*v).write(&mut bytes)?;
        }
        EntityDataValue::Long(v) => {
            v.write(&mut bytes)?;
        }
        EntityDataValue::Float(v) => {
            bytes.extend_from_slice(&v.to_be_bytes());
        }
        EntityDataValue::String(v) => {
            v.write_prefixed::<VarInt>(&mut bytes)?;
        }
        EntityDataValue::Boolean(v) => {
            v.write(&mut bytes)?;
        }
        EntityDataValue::Pose(v) => {
            VarInt(*v as i32).write(&mut bytes)?;
        }
        EntityDataValue::OptionalString(v) => {
            if let Some(s) = v {
                true.write(&mut bytes)?;
                s.write_prefixed::<VarInt>(&mut bytes)?;
            } else {
                false.write(&mut bytes)?;
            }
        }
        EntityDataValue::OptionalTextComponent(v) => {
            if let Some(s) = v {
                true.write(&mut bytes)?;
                // Write as a simple text component in JSON format
                let json = format!(
                    "{{\"text\":\"{}\"}}",
                    s.replace('\\', "\\\\").replace('"', "\\\"")
                );
                json.write_prefixed::<VarInt>(&mut bytes)?;
            } else {
                false.write(&mut bytes)?;
            }
        }
    }

    Ok(bytes)
}

/// Converts a vec of entity data values to packet entries
#[must_use]
pub fn entity_data_to_packet_entries(
    data: Vec<(u8, EntityDataValue)>,
) -> Vec<steel_protocol::packets::game::EntityDataEntry> {
    data.into_iter()
        .filter_map(|(field_id, value)| {
            let serializer_id = value.serializer_id();
            serialize_entity_data_value(&value).ok().map(|value_bytes| {
                steel_protocol::packets::game::EntityDataEntry {
                    field_id,
                    serializer_id,
                    value_bytes,
                }
            })
        })
        .collect()
}
