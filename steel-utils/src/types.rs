// Wrapper types making it harder to accidentaly use the wrong underlying type.

use std::{
    borrow::Cow,
    fmt::{self, Display},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::math::{vector2::Vector2, vector3::Vector3};

// A raw block state id. Using the registry this id can be derived into a block and it's current properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BlockStateId(pub u16);

// A chunk position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos(pub Vector2<i32>);

// A block position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockPos(pub Vector3<i32>);

impl BlockPos {
    // Define constants as per the Java logic, but in Rust style
    const PACKED_HORIZONTAL_LENGTH: u32 = 26;
    const PACKED_Y_LENGTH: u32 = 64 - 2 * Self::PACKED_HORIZONTAL_LENGTH;
    const X_OFFSET: u32 = Self::PACKED_Y_LENGTH + Self::PACKED_HORIZONTAL_LENGTH;
    const Z_OFFSET: u32 = 0;
    const PACKED_X_MASK: i64 = (1i64 << Self::PACKED_HORIZONTAL_LENGTH) - 1;
    const PACKED_Y_MASK: i64 = (1i64 << Self::PACKED_Y_LENGTH) - 1;
    const PACKED_Z_MASK: i64 = (1i64 << Self::PACKED_HORIZONTAL_LENGTH) - 1;

    pub fn as_i64(&self) -> i64 {
        let x = self.0.x as i64;
        let y = self.0.y as i64;
        let z = self.0.z as i64;
        ((x & Self::PACKED_X_MASK) << Self::X_OFFSET)
            | (y & Self::PACKED_Y_MASK)
            | ((z & Self::PACKED_Z_MASK) << Self::Z_OFFSET)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameType {
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 3,
}

impl From<GameType> for i8 {
    fn from(value: GameType) -> Self {
        value as i8
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceLocation {
    pub namespace: Cow<'static, str>,
    pub path: Cow<'static, str>,
}

impl ResourceLocation {
    pub const VANILLA_NAMESPACE: &'static str = "minecraft";

    pub fn vanilla(path: String) -> Self {
        ResourceLocation {
            namespace: Cow::Borrowed(Self::VANILLA_NAMESPACE),
            path: Cow::Owned(path),
        }
    }

    pub const fn vanilla_static(path: &'static str) -> Self {
        ResourceLocation {
            namespace: Cow::Borrowed(Self::VANILLA_NAMESPACE),
            path: Cow::Borrowed(path),
        }
    }

    pub fn valid_namespace_char(namespace_char: char) -> bool {
        namespace_char == '_'
            || namespace_char == '-'
            || namespace_char.is_ascii_lowercase()
            || namespace_char.is_ascii_digit()
            || namespace_char == '.'
    }

    pub fn valid_path_char(path_char: char) -> bool {
        path_char == '_'
            || path_char == '-'
            || path_char.is_ascii_lowercase()
            || path_char.is_ascii_digit()
            || path_char == '/'
            || path_char == '.'
    }

    pub fn validate_namespace(namespace: &str) -> bool {
        namespace.chars().all(Self::valid_namespace_char)
    }

    pub fn validate_path(path: &str) -> bool {
        path.chars().all(Self::valid_path_char)
    }

    pub fn validate(namespace: &str, path: &str) -> bool {
        Self::validate_namespace(namespace) && Self::validate_path(path)
    }
}

impl Display for ResourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

impl FromStr for ResourceLocation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid resource location: {}", s));
        }

        if !ResourceLocation::validate_namespace(parts[0]) {
            return Err(format!("Invalid namespace: {}", parts[0]));
        }

        if !ResourceLocation::validate_path(parts[1]) {
            return Err(format!("Invalid path: {}", parts[1]));
        }

        Ok(ResourceLocation {
            namespace: Cow::Owned(parts[0].to_string()),
            path: Cow::Owned(parts[1].to_string()),
        })
    }
}
impl Serialize for ResourceLocation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ResourceLocation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ResourceLocation::from_str(&s).map_err(serde::de::Error::custom)
    }
}
