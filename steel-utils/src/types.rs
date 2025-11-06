use std::{
    borrow::Cow,
    fmt::{self, Display},
    io::{self, Read, Write},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::{
    math::{vector2::Vector2, vector3::Vector3},
    serial::{ReadFrom, WriteTo},
};

// Usefull for early developement
pub type Todo = ();

// A raw block state id. Using the registry this id can be derived into a block and it's current properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BlockStateId(pub u16);

// A chunk position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos(pub Vector2<i32>);

impl WriteTo for ChunkPos {
    fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        self.0.write(writer)
    }
}

impl ReadFrom for ChunkPos {
    fn read(data: &mut impl Read) -> io::Result<Self> {
        Ok(Self(Vector2::<i32>::read(data)?))
    }
}

// A block position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockPos(pub Vector3<i32>);

impl BlockPos {
    // Define constants as per the Java logic, but in Rust style
    const PACKED_HORIZONTAL_LEN: u32 = 26;
    const PACKED_Y_LEN: u32 = 64 - 2 * Self::PACKED_HORIZONTAL_LEN;
    const X_OFFSET: u32 = Self::PACKED_Y_LEN + Self::PACKED_HORIZONTAL_LEN;
    const Z_OFFSET: u32 = 0;
    const PACKED_X_MASK: i64 = (1i64 << Self::PACKED_HORIZONTAL_LEN) - 1;
    const PACKED_Y_MASK: i64 = (1i64 << Self::PACKED_Y_LEN) - 1;
    const PACKED_Z_MASK: i64 = (1i64 << Self::PACKED_HORIZONTAL_LEN) - 1;

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
pub struct Identifier {
    pub namespace: Cow<'static, str>,
    pub path: Cow<'static, str>,
}

impl Identifier {
    pub const VANILLA_NAMESPACE: &'static str = "minecraft";

    pub fn vanilla(path: String) -> Self {
        Identifier {
            namespace: Cow::Borrowed(Self::VANILLA_NAMESPACE),
            path: Cow::Owned(path),
        }
    }

    pub const fn vanilla_static(path: &'static str) -> Self {
        Identifier {
            namespace: Cow::Borrowed(Self::VANILLA_NAMESPACE),
            path: Cow::Borrowed(path),
        }
    }

    pub fn valid_namespace_char(char: char) -> bool {
        char == '_'
            || char == '-'
            || char.is_ascii_lowercase()
            || char.is_ascii_digit()
            || char == '.'
    }

    pub fn valid_char(char: char) -> bool {
        Self::valid_namespace_char(char) || char == '/'
    }

    pub fn validate_namespace(namespace: &str) -> bool {
        namespace.chars().all(Self::valid_namespace_char)
    }

    pub fn validate_path(path: &str) -> bool {
        path.chars().all(Self::valid_char)
    }

    pub fn validate(namespace: &str, path: &str) -> bool {
        Self::validate_namespace(namespace) && Self::validate_path(path)
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

impl FromStr for Identifier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid resource location: {}", s));
        }

        if !Identifier::validate_namespace(parts[0]) {
            return Err(format!("Invalid namespace: {}", parts[0]));
        }

        if !Identifier::validate_path(parts[1]) {
            return Err(format!("Invalid path: {}", parts[1]));
        }

        Ok(Identifier {
            namespace: Cow::Owned(parts[0].to_string()),
            path: Cow::Owned(parts[1].to_string()),
        })
    }
}
impl Serialize for Identifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Identifier::from_str(&s).map_err(serde::de::Error::custom)
    }
}
