use steel_macros::PacketRead;
use uuid::Uuid;

#[derive(PacketRead, Clone, Debug)]
pub struct SHelloPacket {
    #[read_as(as = "string", bound = 16)]
    pub name: String,
    pub profile_id: Uuid,
}
