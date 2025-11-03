use serde::{Deserialize, Serialize};
use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::clientbound::login::CLIENTBOUND_LOGIN_FINISHED;
use uuid::Uuid;

#[derive(Clone, Debug, WriteTo, Serialize, Deserialize)]
pub struct GameProfileProperty {
    #[write_as(as = "string", bound = 16)]
    pub name: String,
    #[write_as(as = "string")]
    pub value: String,
    #[write_as(as = "string")]
    pub signature: Option<String>,
}

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(LOGIN = "CLIENTBOUND_LOGIN_FINISHED")]
pub struct CLoginFinishedPacket {
    pub uuid: Uuid,
    #[write_as(as = "string", bound = 16)]
    pub name: String,
    #[write_as(as = "vec")]
    pub properties: Vec<GameProfileProperty>,
}

impl CLoginFinishedPacket {
    pub fn new(uuid: Uuid, name: String, properties: Vec<GameProfileProperty>) -> Self {
        Self {
            uuid,
            name,
            properties,
        }
    }
}
