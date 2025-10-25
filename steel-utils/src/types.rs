// Wrapper types making it harder to accidentaly use the wrong underlying type.

use std::{
    borrow::Cow,
    fmt::{self, Display},
    io,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    pin::Pin,
    str::FromStr,
    task::{Context, Poll},
};

use tokio::io::AsyncWrite;

use crate::math::{vector2::Vector2, vector3::Vector3};

// A raw block state id. Using the registry this id can be derived into a block and it's current properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockStateId(pub u16);

// A chunk position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos(pub Vector2<i32>);

// A block position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockPos(pub Vector3<i32>);

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

/// Its like a vec but with reserveable front space.
/// Its meant for our packet serialization,
/// you can just put the len of the packet in front without reallocating
/// keep in mind that calling multiple set_in_front() sets the data in reverse order compared to extend_from_slice()
pub struct FrontVec {
    buf: Vec<u8>,
    front_space: usize,
}

impl FrontVec {
    pub fn capacity(reserve: usize, capacity: usize) -> Self {
        let total = reserve + capacity;
        let mut buf = Vec::with_capacity(total);

        #[allow(invalid_value)]
        buf.resize_with(total, || unsafe { MaybeUninit::uninit().assume_init() });

        Self {
            buf,
            front_space: reserve,
        }
    }

    pub fn new(reserve: usize) -> Self {
        let mut buf = Vec::with_capacity(reserve);

        #[allow(invalid_value)]
        buf.resize_with(reserve, || unsafe { MaybeUninit::uninit().assume_init() });

        Self {
            buf,
            front_space: reserve,
        }
    }

    pub const fn len(&self) -> usize {
        self.buf.len() - self.front_space
    }

    pub fn push(&mut self, value: u8) {
        self.buf.push(value);
    }

    pub fn extend_from_slice(&mut self, other: &[u8]) {
        self.buf.extend_from_slice(other);
    }

    #[track_caller]
    pub fn set_in_front(&mut self, other: &[u8]) {
        if self.front_space < other.len() {
            panic!("Not enough reserved space");
        }

        let new_start = self.front_space - other.len();
        self.buf[new_start..self.front_space].copy_from_slice(other);
        self.front_space = new_start;
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buf[self.front_space..self.buf.len()]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        let len = self.buf.len();
        &mut self.buf[self.front_space..len]
    }
}

impl io::Write for FrontVec {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl AsyncWrite for FrontVec {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let this = self.get_mut();
        this.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl Deref for FrontVec {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for FrontVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}
