//! Component value newtypes with proper network encoding.
//!
//! These types wrap primitive values and implement `WriteTo` and `ReadFrom`
//! with the correct encoding for the Minecraft protocol.

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
