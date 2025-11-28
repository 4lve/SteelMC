use std::{
    borrow::Cow,
    fmt::{self, Display},
    io::{self, Read, Write},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::{
    math::{Vector2, Vector3},
    serial::{ReadFrom, WriteTo},
};

// Useful for early development
/// A type alias for `()`. Useful for early development.
pub type Todo = ();

/// A raw block state id. Using the registry this id can be derived into a block and it's current properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BlockStateId(pub u16);

/// A chunk position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChunkPos(pub Vector2<i32>);

impl std::hash::Hash for ChunkPos {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.as_i64() as u64);
    }
}

impl ChunkPos {
    #[must_use]
    /// Creates a new `ChunkPos` with the given x and y coordinates.
    pub const fn new(x: i32, y: i32) -> Self {
        Self(Vector2::new(x, y))
    }

    /// Converts the `ChunkPos` to an `i64`.
    #[must_use]
    #[inline]
    pub fn as_i64(&self) -> i64 {
        (i64::from(self.0.x) & 0xFFFF_FFFF) | ((i64::from(self.0.y) & 0xFFFF_FFFF) << 32)
    }

    /// Creates a new `ChunkPos` from an `i64`.
    #[must_use]
    #[inline]
    pub fn from_i64(value: i64) -> Self {
        Self(Vector2::new(
            (value & 0xFFFF_FFFF) as i32,
            (value >> 32) as i32,
        ))
    }
}

#[allow(missing_docs)]
impl WriteTo for ChunkPos {
    fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        self.0.write(writer)
    }
}

#[allow(missing_docs)]
impl ReadFrom for ChunkPos {
    fn read(data: &mut impl Read) -> io::Result<Self> {
        Ok(Self(Vector2::<i32>::read(data)?))
    }
}

/// A block position.
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

    /// Converts the `BlockPos` to an `i64`.
    #[must_use]
    pub fn as_i64(&self) -> i64 {
        let x = i64::from(self.0.x);
        let y = i64::from(self.0.y);
        let z = i64::from(self.0.z);
        ((x & Self::PACKED_X_MASK) << Self::X_OFFSET)
            | (y & Self::PACKED_Y_MASK)
            | ((z & Self::PACKED_Z_MASK) << Self::Z_OFFSET)
    }
}

/// The game type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum GameType {
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 3,
}

#[allow(missing_docs)]
impl From<GameType> for i8 {
    fn from(value: GameType) -> Self {
        value as i8
    }
}

/// An identifier used by Minecraft.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    /// The namespace of the identifier.
    pub namespace: Cow<'static, str>,
    /// The path of the identifier.
    pub path: Cow<'static, str>,
}

impl Identifier {
    /// The vanilla namespace.
    pub const VANILLA_NAMESPACE: &'static str = "minecraft";

    /// Creates a new `Identifier` with the vanilla namespace.
    #[must_use]
    pub fn vanilla(path: String) -> Self {
        Identifier {
            namespace: Cow::Borrowed(Self::VANILLA_NAMESPACE),
            path: Cow::Owned(path),
        }
    }

    /// Creates a new `Identifier` with the vanilla namespace and a static path.
    #[must_use]
    pub const fn vanilla_static(path: &'static str) -> Self {
        Identifier {
            namespace: Cow::Borrowed(Self::VANILLA_NAMESPACE),
            path: Cow::Borrowed(path),
        }
    }

    /// Returns whether the character is a valid namespace character.
    #[must_use]
    pub fn valid_namespace_char(char: char) -> bool {
        char == '_'
            || char == '-'
            || char.is_ascii_lowercase()
            || char.is_ascii_digit()
            || char == '.'
    }

    /// Returns whether the character is a valid path character.
    #[must_use]
    pub fn valid_char(char: char) -> bool {
        Self::valid_namespace_char(char) || char == '/'
    }

    /// Returns whether the namespace is valid.
    pub fn validate_namespace(namespace: &str) -> bool {
        namespace.chars().all(Self::valid_namespace_char)
    }

    /// Returns whether the path is valid.
    pub fn validate_path(path: &str) -> bool {
        path.chars().all(Self::valid_char)
    }

    /// Returns whether the namespace and path are valid.
    #[must_use]
    pub fn validate(namespace: &str, path: &str) -> bool {
        Self::validate_namespace(namespace) && Self::validate_path(path)
    }
}

#[allow(missing_docs)]
impl Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

#[allow(missing_docs)]
impl FromStr for Identifier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid resource location: {s}"));
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
#[allow(missing_docs)]
impl Serialize for Identifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[allow(missing_docs)]
impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Identifier::from_str(&s).map_err(serde::de::Error::custom)
    }
}
