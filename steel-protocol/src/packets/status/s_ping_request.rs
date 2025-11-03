use steel_macros::PacketRead;

#[derive(PacketRead, Clone, Debug)]
pub struct SPingRequestPacket {
    pub time: i64,
}
