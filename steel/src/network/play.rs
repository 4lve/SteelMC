use steel_protocol::packets::common::SCustomPayload;

use crate::network::java_tcp_client::JavaTcpClient;

pub fn handle_custom_payload(_tcp_client: &JavaTcpClient, packet: SCustomPayload) {
    println!("Custom payload packet: {:?}", packet);
}
