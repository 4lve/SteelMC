//! Component value newtypes with proper network encoding.
//!
//! These types wrap primitive values and implement `WriteTo` and `ReadFrom`
//! with the correct encoding for the Minecraft protocol.

use simdnbt::{FromNbtTag, ToNbtTag, borrow::NbtTag as BorrowedNbtTag, owned::NbtTag};
use steel_macros::{ReadFrom, WriteTo};
use steel_utils::hash::{ComponentHasher, HashComponent};

/// Damage value for items. Encoded as VarInt on the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, WriteTo, ReadFrom)]
#[write(as = VarInt)]
#[read(as = VarInt)]
pub struct Damage(pub i32);

impl HashComponent for Damage {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        hasher.put_int(self.0);
    }
}

impl ToNbtTag for Damage {
    fn to_nbt_tag(self) -> NbtTag {
        NbtTag::Int(self.0)
    }
}

impl FromNbtTag for Damage {
    fn from_nbt_tag(tag: BorrowedNbtTag) -> Option<Self> {
        Some(Self(tag.int()?))
    }
}

/// Max damage value for items. Encoded as VarInt on the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, WriteTo, ReadFrom)]
#[write(as = VarInt)]
#[read(as = VarInt)]
pub struct MaxDamage(pub i32);

impl HashComponent for MaxDamage {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        hasher.put_int(self.0);
    }
}

impl ToNbtTag for MaxDamage {
    fn to_nbt_tag(self) -> NbtTag {
        NbtTag::Int(self.0)
    }
}

impl FromNbtTag for MaxDamage {
    fn from_nbt_tag(tag: BorrowedNbtTag) -> Option<Self> {
        Some(Self(tag.int()?))
    }
}

/// Max stack size for items. Encoded as VarInt on the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, WriteTo, ReadFrom)]
#[write(as = VarInt)]
#[read(as = VarInt)]
pub struct MaxStackSize(pub i32);

impl HashComponent for MaxStackSize {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        hasher.put_int(self.0);
    }
}

impl ToNbtTag for MaxStackSize {
    fn to_nbt_tag(self) -> NbtTag {
        NbtTag::Int(self.0)
    }
}

impl FromNbtTag for MaxStackSize {
    fn from_nbt_tag(tag: BorrowedNbtTag) -> Option<Self> {
        Some(Self(tag.int()?))
    }
}

/// Repair cost for items. Encoded as VarInt on the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, WriteTo, ReadFrom)]
#[write(as = VarInt)]
#[read(as = VarInt)]
pub struct RepairCost(pub i32);

impl HashComponent for RepairCost {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        hasher.put_int(self.0);
    }
}

impl ToNbtTag for RepairCost {
    fn to_nbt_tag(self) -> NbtTag {
        NbtTag::Int(self.0)
    }
}

impl FromNbtTag for RepairCost {
    fn from_nbt_tag(tag: BorrowedNbtTag) -> Option<Self> {
        Some(Self(tag.int()?))
    }
}
