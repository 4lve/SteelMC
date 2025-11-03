use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::clientbound::login::CLIENTBOUND_LOGIN_DISCONNECT;
use steel_utils::text::TextComponent;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(LOGIN = "CLIENTBOUND_LOGIN_DISCONNECT")]
pub struct CLoginDisconnectPacket {
    #[write_as(as = "json")]
    pub reason: TextComponent,
}

impl CLoginDisconnectPacket {
    pub fn new(reason: TextComponent) -> Self {
        Self { reason }
    }
}
