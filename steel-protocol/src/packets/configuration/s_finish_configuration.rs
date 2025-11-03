use steel_macros::PacketRead;

#[derive(PacketRead, Clone, Debug)]
pub struct SFinishConfigurationPacket {}
