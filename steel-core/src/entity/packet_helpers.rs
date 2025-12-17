//! Helper functions for converting entity data to packet format

use super::{EntityDataSerializers, EntityDataValue, Pose};
use steel_utils::codec::VarInt;
use steel_utils::serial::{PrefixedWrite, WriteTo};

/// Converts entity data to packet-ready format
pub fn serialize_entity_data_value(value: &EntityDataValue) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();

    match value.serializer_id() {
        EntityDataSerializers::BYTE => {
            if let Some(v) = value.get::<u8>() {
                v.write(&mut bytes)?;
            }
        }
        EntityDataSerializers::INT => {
            if let Some(v) = value.get::<i32>() {
                VarInt(v).write(&mut bytes)?;
            }
        }
        EntityDataSerializers::LONG => {
            if let Some(v) = value.get::<i64>() {
                v.write(&mut bytes)?;
            }
        }
        EntityDataSerializers::FLOAT => {
            if let Some(v) = value.get::<f32>() {
                bytes.extend_from_slice(&v.to_be_bytes());
            }
        }
        EntityDataSerializers::STRING => {
            if let Some(v) = value.get::<String>() {
                v.write_prefixed::<VarInt>(&mut bytes)?;
            }
        }
        EntityDataSerializers::BOOLEAN => {
            if let Some(v) = value.get::<bool>() {
                v.write(&mut bytes)?;
            }
        }
        EntityDataSerializers::POSE => {
            if let Some(v) = value.get::<Pose>() {
                VarInt(v as i32).write(&mut bytes)?;
            }
        }
        EntityDataSerializers::OPTIONAL_STRING => {
            if let Some(v) = value.get::<Option<String>>() {
                if let Some(s) = v {
                    true.write(&mut bytes)?;
                    s.write_prefixed::<VarInt>(&mut bytes)?;
                } else {
                    false.write(&mut bytes)?;
                }
            }
        }
        EntityDataSerializers::OPTIONAL_TEXT_COMPONENT => {
            if let Some(v) = value.get::<Option<String>>() {
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
        _ => {
            log::warn!(
                "Unsupported entity data serializer: {}",
                value.serializer_id()
            );
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
