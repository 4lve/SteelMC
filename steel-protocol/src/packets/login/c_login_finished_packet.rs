use steel_macros::PacketWrite;
use uuid::Uuid;

#[derive(Clone, Debug, PacketWrite)]
pub struct Property {
    #[write_as(as = "string", bound = 16)]
    pub name: String,
    #[write_as(as = "string")]
    pub value: String,
    #[write_as(as = "string")]
    pub signature: Option<String>,
}

#[derive(PacketWrite, Clone, Debug)]
pub struct CLoginFinishedPacket {
    pub uuid: Uuid,
    #[write_as(as = "string", bound = 16)]
    pub name: String,
    #[write_as(as = "vec")]
    pub properties: Vec<Property>,
}

impl CLoginFinishedPacket {
    pub fn new(uuid: Uuid, name: String, properties: Vec<Property>) -> Self {
        Self {
            uuid,
            name,
            properties,
        }
    }
}
