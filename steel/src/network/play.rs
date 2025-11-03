use steel_protocol::packets::common::s_custom_payload::SCustomPayloadPacket;

use crate::network::java_tcp_client::JavaTcpClient;

pub fn handle_custom_payload(_tcp_client: &JavaTcpClient, packet: &SCustomPayloadPacket) {
    println!("Custom payload packet: {:?}", packet);
}
