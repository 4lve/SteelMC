use serde::{Deserialize, Serialize};
use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::login::C_LOGIN_FINISHED;
use uuid::Uuid;

#[derive(Clone, Debug, WriteTo, Serialize, Deserialize)]
pub struct GameProfileProperty {
    #[write(as = Prefixed(VarInt), bound = 16)]
    pub name: String,
    #[write(as = Prefixed(VarInt))]
    pub value: String,
    #[write(as = Prefixed(VarInt))]
    pub signature: Option<String>,
}

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Login = C_LOGIN_FINISHED)]
pub struct CLoginFinished<'a> {
    pub uuid: Uuid,
    #[write(as = Prefixed(VarInt), bound = 16)]
    pub name: &'a str,
    #[write(as = Prefixed(VarInt))]
    pub properties: &'a [GameProfileProperty],
}

impl<'a> CLoginFinished<'a> {
    #[must_use]
    pub fn new(uuid: Uuid, name: &'a str, properties: &'a [GameProfileProperty]) -> Self {
        Self {
            uuid,
            name,
            properties,
        }
    }
}
