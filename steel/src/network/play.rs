use steel_protocol::packets::common::SCustomPayload;

use crate::network::java_tcp_client::JavaTcpClient;

impl JavaTcpClient {
    pub fn handle_custom_payload(&self, packet: SCustomPayload) {
        println!("Custom payload packet: {:?}", packet);
    }
}
